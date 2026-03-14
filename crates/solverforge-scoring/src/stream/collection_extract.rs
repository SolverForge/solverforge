// CollectionExtract trait for ergonomic entity collection extraction.
//
// Allows extractor closures to return either `&[A]` or `&Vec<A>`,
// so users can write `vec(|s| &s.employees)` without `.as_slice()`.
//
// # Usage
//
// ```
// use solverforge_scoring::stream::collection_extract::{CollectionExtract, VecExtract, vec};
//
// struct Schedule { employees: Vec<String> }
//
// // Direct slice closure — works out of the box:
// let e1 = |s: &Schedule| s.employees.as_slice();
// let _: &[String] = e1.extract(&Schedule { employees: vec![] });
//
// // Vec reference closure — wrap with `vec(...)`:
// let e2 = vec(|s: &Schedule| &s.employees);
// let _: &[String] = e2.extract(&Schedule { employees: vec![] });
// ```

// Extracts a slice of entities from the solution.
//
// The associated type `Item` names the entity type, allowing callers to
// write `E: CollectionExtract<S, Item = A>` when `A` must be inferred from `E`
// rather than stated as a separate generic parameter.
pub trait CollectionExtract<S>: Send + Sync {
    // The entity type yielded by this extractor.
    type Item;

    // Extracts the entity slice from the solution.
    fn extract<'s>(&self, s: &'s S) -> &'s [Self::Item];
}

impl<S, A, F> CollectionExtract<S> for F
where
    F: Fn(&S) -> &[A] + Send + Sync,
{
    type Item = A;

    #[inline]
    fn extract<'s>(&self, s: &'s S) -> &'s [A] {
        self(s)
    }
}

// Wraps a `Fn(&S) -> &Vec<A>` closure so it satisfies `CollectionExtract<S>`.
//
// Construct via the [`vec`] free function.
pub struct VecExtract<F>(pub F);

impl<S, A, F> CollectionExtract<S> for VecExtract<F>
where
    F: Fn(&S) -> &Vec<A> + Send + Sync,
{
    type Item = A;

    #[inline]
    fn extract<'s>(&self, s: &'s S) -> &'s [A] {
        (self.0)(s).as_slice()
    }
}

// Wraps a `Fn(&S) -> &Vec<A>` closure into a [`VecExtract`] that satisfies
// [`CollectionExtract<S>`].
//
// Use this when your solution field is a `Vec<A>` and you want to write
// `|s| &s.field` instead of `|s| s.field.as_slice()`.
//
// # Example
//
// ```
// use solverforge_scoring::stream::collection_extract::{CollectionExtract, vec};
//
// struct Schedule { employees: Vec<String> }
//
// let extractor = vec(|s: &Schedule| &s.employees);
// let schedule = Schedule { employees: vec!["Alice".into()] };
// assert_eq!(extractor.extract(&schedule), &["Alice".to_string()]);
// ```
pub fn vec<S, A, F>(f: F) -> VecExtract<F>
where
    F: Fn(&S) -> &Vec<A> + Send + Sync,
{
    VecExtract(f)
}
