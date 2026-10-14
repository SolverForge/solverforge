//! Non-Copy payload transfer through the shared list kernels.
//!
//! A Clone-only (no `Copy`) element must flow through change, swap, permute,
//! and ruin do/undo by ownership, with duplicate-equal payloads carrying
//! distinct positions intact.

use super::*;
use smallvec::smallvec;

#[derive(Clone, Debug, PartialEq)]
struct Goods {
    id: usize,
    tag: String,
}

#[derive(Clone, Debug)]
struct Bin {
    items: Vec<Goods>,
}

#[derive(Clone, Debug)]
struct GoodsPlan {
    bins: Vec<Bin>,
    score: Option<SoftScore>,
}

impl PlanningSolution for GoodsPlan {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn goods(id: usize) -> Goods {
    Goods {
        id,
        tag: format!("g{id}"),
    }
}

fn get_bins(s: &GoodsPlan) -> &Vec<Bin> {
    &s.bins
}
fn get_bins_mut(s: &mut GoodsPlan) -> &mut Vec<Bin> {
    &mut s.bins
}

fn item_len(s: &GoodsPlan, entity_idx: usize) -> usize {
    s.bins.get(entity_idx).map_or(0, |bin| bin.items.len())
}

fn item_get(s: &GoodsPlan, entity_idx: usize, pos: usize) -> Option<Goods> {
    s.bins
        .get(entity_idx)
        .and_then(|bin| bin.items.get(pos))
        .cloned()
}

fn item_remove(s: &mut GoodsPlan, entity_idx: usize, pos: usize) -> Option<Goods> {
    s.bins.get_mut(entity_idx).map(|bin| bin.items.remove(pos))
}

fn item_insert(s: &mut GoodsPlan, entity_idx: usize, pos: usize, val: Goods) {
    if let Some(bin) = s.bins.get_mut(entity_idx) {
        bin.items.insert(pos, val);
    }
}

fn item_set(s: &mut GoodsPlan, entity_idx: usize, pos: usize, val: Goods) {
    if let Some(bin) = s.bins.get_mut(entity_idx) {
        if let Some(item) = bin.items.get_mut(pos) {
            *item = val;
        }
    }
}

fn ruin_item_remove(s: &mut GoodsPlan, entity_idx: usize, pos: usize) -> Goods {
    s.bins[entity_idx].items.remove(pos)
}

fn sublist_remove(s: &mut GoodsPlan, entity_idx: usize, start: usize, end: usize) -> Vec<Goods> {
    let mut items = std::mem::take(&mut s.bins[entity_idx].items);
    let removed: Vec<Goods> = items.drain(start..end).collect();
    s.bins[entity_idx].items = items;
    removed
}

fn sublist_insert(s: &mut GoodsPlan, entity_idx: usize, pos: usize, values: Vec<Goods>) {
    let items = &mut s.bins[entity_idx].items;
    for (offset, value) in values.into_iter().enumerate() {
        items.insert(pos + offset, value);
    }
}

fn create_director(bins: Vec<Vec<Goods>>) -> ScoreDirector<GoodsPlan, ()> {
    let solution = GoodsPlan {
        bins: bins.into_iter().map(|items| Bin { items }).collect(),
        score: None,
    };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Bin",
        "bins",
        get_bins,
        get_bins_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Bin", TypeId::of::<Bin>(), "bins").with_extractor(extractor);
    let descriptor =
        SolutionDescriptor::new("GoodsPlan", TypeId::of::<GoodsPlan>()).with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.bins.len())
}

#[test]
fn change_transfers_clone_only_payload_exactly() {
    let mut director = create_director(vec![vec![goods(0), goods(1)], vec![goods(2)]]);
    let mov = ListChangeMove::<GoodsPlan, Goods>::new(
        0,
        1,
        1,
        0,
        item_len,
        item_get,
        item_remove,
        item_insert,
        "items",
        0,
    );
    assert!(mov.is_doable(&director));
    {
        let mut recording = SnapshotDirector::new(&mut director);
        mov.do_move(&mut recording);
        assert_eq!(
            director.working_solution().bins[0].items,
            [goods(0)],
            "the moved item left the source without leaving a clone behind"
        );
        assert_eq!(
            director.working_solution().bins[1].items,
            [goods(1), goods(2)]
        );
        recording.undo_changes();
    }
    assert_eq!(
        director.working_solution().bins[0].items,
        vec![goods(0), goods(1)]
    );
    assert_eq!(director.working_solution().bins[1].items, vec![goods(2)]);
}

#[test]
fn swap_exchanges_clone_only_payload_without_duplicating() {
    let mut director = create_director(vec![vec![goods(0), goods(1), goods(2)]]);
    let mov =
        ListSwapMove::<GoodsPlan, Goods>::new(0, 0, 0, 2, item_len, item_get, item_set, "items", 0);
    assert!(mov.is_doable(&director));
    {
        let mut recording = SnapshotDirector::new(&mut director);
        mov.do_move(&mut recording);
        assert_eq!(
            director.working_solution().bins[0].items,
            [goods(2), goods(1), goods(0)]
        );
        recording.undo_changes();
    }
    assert_eq!(
        director.working_solution().bins[0].items,
        vec![goods(0), goods(1), goods(2)]
    );
}

#[test]
fn duplicate_equal_payloads_keep_distinct_coordinates() {
    // Two equal-valued payloads at distinct positions: a swap between equal
    // payloads is not doable by the value-comparison policy.
    let director = create_director(vec![vec![goods(5), goods(5), goods(1)]]);
    let mov =
        ListSwapMove::<GoodsPlan, Goods>::new(0, 0, 0, 1, item_len, item_get, item_set, "items", 0);
    assert!(
        !mov.is_doable(&director),
        "a swap between equal payloads is not doable by value-comparison policy"
    );
    // Distinct payloads across bins still swap by coordinates: bin1 gains
    // bin0's last item and vice versa.
    let mut director = create_director(vec![vec![goods(7), goods(5)], vec![goods(9), goods(6)]]);
    let cross =
        ListSwapMove::<GoodsPlan, Goods>::new(0, 1, 1, 0, item_len, item_get, item_set, "items", 0);
    assert!(cross.is_doable(&director));
    {
        let mut recording = SnapshotDirector::new(&mut director);
        cross.do_move(&mut recording);
        assert_eq!(
            director.working_solution().bins[0].items,
            [goods(7), goods(9)]
        );
        assert_eq!(
            director.working_solution().bins[1].items,
            [goods(5), goods(6)]
        );
        recording.undo_changes();
    }
    assert_eq!(
        director.working_solution().bins[0].items,
        vec![goods(7), goods(5)]
    );
    assert_eq!(
        director.working_solution().bins[1].items,
        vec![goods(9), goods(6)]
    );
}

#[test]
fn permute_moves_clone_only_values_once_and_undo_restores() {
    let mut director =
        create_director(vec![vec![goods(0), goods(1), goods(2), goods(3), goods(4)]]);
    let mov = ListPermuteMove::<GoodsPlan, Goods>::new(
        0,
        1,
        4,
        smallvec![2, 0, 1],
        item_len,
        item_get,
        sublist_remove,
        sublist_insert,
        "items",
        0,
    );
    assert!(mov.is_doable(&director));
    {
        let mut recording = SnapshotDirector::new(&mut director);
        let undo = mov.do_move(&mut recording);
        assert_eq!(
            director.working_solution().bins[0].items,
            [goods(0), goods(3), goods(1), goods(2), goods(4)]
        );
        assert_eq!(
            undo.as_slice(),
            &[1, 2, 0],
            "the permute undo is the inverse permutation, not a value segment"
        );
        mov.undo_move(&mut director, undo);
        assert_eq!(
            director.working_solution().bins[0].items,
            vec![goods(0), goods(1), goods(2), goods(3), goods(4)]
        );
    }
    assert_eq!(
        director.working_solution().bins[0].items,
        vec![goods(0), goods(1), goods(2), goods(3), goods(4)]
    );
}

#[test]
fn ruin_moves_clone_only_values_by_ownership_and_undo_is_exact() {
    let mut director = create_director(vec![vec![goods(0), goods(1)], vec![goods(2)], vec![]]);
    let mov = ListRuinMove::<GoodsPlan, Goods>::new(
        0,
        &[0, 1],
        |s| s.bins.len(),
        item_len,
        item_get,
        ruin_item_remove,
        item_insert,
        "items",
        0,
    );
    assert!(mov.is_doable(&director));
    let undo = mov.do_move(&mut director);
    assert_eq!(
        undo.len(),
        2,
        "both ruined items must pass through the placement bookkeeping"
    );
    mov.undo_move(&mut director, undo);
    assert_eq!(
        director.working_solution().bins[0].items,
        vec![goods(0), goods(1)]
    );
    assert_eq!(director.working_solution().bins[1].items, vec![goods(2)]);
    assert_eq!(
        director.working_solution().bins[2].items,
        Vec::<Goods>::new()
    );
}
