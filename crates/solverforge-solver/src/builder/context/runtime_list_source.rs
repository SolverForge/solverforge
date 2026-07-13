//! Per-run declared-list source binding shared by canonical construction.
//!
//! A compiled graph owns only structural slot metadata. The declared element
//! stream belongs to the imported solution for one solve, so this binder
//! freezes it once and resolves every assigned value by its stable source key.
//! Construction kernels therefore never need payload equality or hash scans.

use super::list_access::ListAccess;
use std::collections::HashMap;

/// The source-stream operations required by a construction binder.
///
/// `ListAccess` implements this automatically. The public Clarke-Wright
/// adapter also implements it directly, which keeps its established function
/// pointer constructor on the same source-key protocol as compiled slots.
pub(crate) trait ListSourceAccess<S> {
    type Element: Clone + Send + Sync + 'static;

    fn element_count(&self, solution: &S) -> usize;
    fn index_to_element(&self, solution: &S, source_index: usize) -> Option<Self::Element>;
    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize;
    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element>;
}

/// The current-assignment half of a previously bound list source.
///
/// This deliberately omits declaration enumeration. A construction phase that
/// already owns a `RuntimeListSourceIndex` may refresh its unassigned stream
/// after an earlier phase has committed work, without invoking `element_count`
/// or `index_to_element` again.
pub(crate) trait AssignedListSourceAccess<S> {
    type Element: Clone + Send + Sync + 'static;

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize;
    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element>;
}

impl<S, A> AssignedListSourceAccess<S> for A
where
    A: ListSourceAccess<S>,
{
    type Element = A::Element;

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize {
        ListSourceAccess::element_source_key(self, solution, element)
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        ListSourceAccess::assigned_elements(self, solution)
    }
}

impl<S, A> ListSourceAccess<S> for A
where
    A: ListAccess<S>,
{
    type Element = A::Element;

    fn element_count(&self, solution: &S) -> usize {
        ListAccess::element_count(self, solution)
    }

    fn index_to_element(&self, solution: &S, source_index: usize) -> Option<Self::Element> {
        ListAccess::index_to_element(self, solution, source_index)
    }

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize {
        ListAccess::element_source_key(self, solution, element)
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        ListAccess::assigned_elements(self, solution)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListConstructionKernelError {
    MissingDeclaredElement {
        source_index: usize,
    },
    DuplicateDeclaredElement {
        first_source_index: usize,
        duplicate_source_index: usize,
    },
    AssignedElementNotDeclared {
        assigned_occurrence: usize,
    },
    DuplicateAssignedElement {
        source_index: usize,
        first_assigned_occurrence: usize,
        duplicate_assigned_occurrence: usize,
    },
}

/// Immutable source-key resolver bound to one declared stream and one solve.
///
/// It deliberately is not held in a compiled graph: values may change between
/// imported solutions, while the graph remains reusable. The element payloads
/// are frozen here, so a construction phase never rereads the source stream.
#[derive(Debug)]
pub(crate) struct RuntimeListSourceIndex<E> {
    source_index_by_key: HashMap<usize, usize>,
    elements: Vec<E>,
}

/// The fully validated declared stream for one construction invocation.
///
/// Binding validates both declarations and the current assigned values before
/// phase work begins. A prepared runtime request can therefore surface a
/// typed bind failure before it creates a `Phase` or publishes progress.
pub(crate) struct RuntimeListSourceBinding<E> {
    source_index: RuntimeListSourceIndex<E>,
    initial_unassigned: Vec<SourceElement<E>>,
}

impl<E> RuntimeListSourceBinding<E> {
    /// Consumes this validation result and retains the frozen declaration
    /// index. Callers that do not execute immediately intentionally discard
    /// the initial unassigned snapshot and refresh from the cached index.
    pub(crate) fn into_source_index(self) -> RuntimeListSourceIndex<E> {
        self.source_index
    }

    /// Consumes the validation result without repeating current-assignment
    /// resolution at the first reached prepared boundary.
    pub(crate) fn into_parts(self) -> (RuntimeListSourceIndex<E>, Vec<SourceElement<E>>) {
        (self.source_index, self.initial_unassigned)
    }
}

impl<E> RuntimeListSourceIndex<E> {
    pub(crate) fn bind<S, A>(access: &A, solution: &S) -> Result<Self, ListConstructionKernelError>
    where
        A: ListSourceAccess<S, Element = E>,
        E: Clone + Send + Sync + 'static,
    {
        let source_count = access.element_count(solution);
        let mut source_index_by_key = HashMap::with_capacity(source_count);
        let mut elements = Vec::with_capacity(source_count);
        for source_index in 0..source_count {
            let element = access
                .index_to_element(solution, source_index)
                .ok_or(ListConstructionKernelError::MissingDeclaredElement { source_index })?;
            let key = access.element_source_key(solution, &element);
            if let Some(first_source_index) = source_index_by_key.insert(key, source_index) {
                return Err(ListConstructionKernelError::DuplicateDeclaredElement {
                    first_source_index,
                    duplicate_source_index: source_index,
                });
            }
            elements.push(element);
        }
        Ok(Self {
            source_index_by_key,
            elements,
        })
    }

    pub(crate) fn source_index_for_key(&self, key: usize) -> Option<usize> {
        self.source_index_by_key.get(&key).copied()
    }

    pub(crate) fn source_count(&self) -> usize {
        self.elements.len()
    }

    pub(crate) fn element(&self, source_index: usize) -> &E {
        &self.elements[source_index]
    }
}

/// One unassigned declaration, addressed solely by its frozen source index.
#[derive(Clone)]
pub(crate) struct SourceElement<E> {
    pub(crate) source_index: usize,
    pub(crate) element: E,
}

/// Resolves current assignments against a frozen declaration binding.
///
/// This function never reads `element_count` or `index_to_element`. It derives
/// remaining entries from the immutable source index plus the solution's
/// current assigned values, so multiple configured construction phases can
/// share one declaration snapshot without reinserting stale work.
pub(crate) fn unassigned_from_current_assignment<S, A>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    solution: &S,
) -> Result<Vec<SourceElement<A::Element>>, ListConstructionKernelError>
where
    A: AssignedListSourceAccess<S>,
{
    let mut assigned = vec![false; source_index.source_count()];
    let mut first_assigned_occurrence = vec![None; source_index.source_count()];
    for (assigned_occurrence, value) in access.assigned_elements(solution).into_iter().enumerate() {
        let key = access.element_source_key(solution, &value);
        let Some(resolved_source_index) = source_index.source_index_for_key(key) else {
            return Err(ListConstructionKernelError::AssignedElementNotDeclared {
                assigned_occurrence,
            });
        };
        if let Some(first_assigned_occurrence) = first_assigned_occurrence[resolved_source_index] {
            return Err(ListConstructionKernelError::DuplicateAssignedElement {
                source_index: resolved_source_index,
                first_assigned_occurrence,
                duplicate_assigned_occurrence: assigned_occurrence,
            });
        }
        first_assigned_occurrence[resolved_source_index] = Some(assigned_occurrence);
        assigned[resolved_source_index] = true;
    }

    let mut elements = Vec::with_capacity(source_index.source_count());
    for (declared_source_index, is_assigned) in assigned.into_iter().enumerate() {
        if !is_assigned {
            elements.push(SourceElement {
                source_index: declared_source_index,
                element: source_index.element(declared_source_index).clone(),
            });
        }
    }
    Ok(elements)
}

/// Binds declared and assigned source state together before phase execution.
pub(crate) fn bind_runtime_list_source<S, A>(
    access: &A,
    solution: &S,
) -> Result<RuntimeListSourceBinding<A::Element>, ListConstructionKernelError>
where
    A: ListSourceAccess<S>,
{
    let source_index = RuntimeListSourceIndex::bind(access, solution)?;
    let initial_unassigned = unassigned_from_current_assignment(access, &source_index, solution)?;
    Ok(RuntimeListSourceBinding {
        source_index,
        initial_unassigned,
    })
}
