use itertools::iproduct;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Sub};

const N: usize = 4;
const EPS: f64 = 0.05;

// How many decimal places are necessary before two values are considered equal
const PRECISION: u32 = 6;

#[derive(Clone, Copy, Debug)]
pub enum Cmp {
    Gte,
    Lt,
}

#[derive(Clone, Copy, Debug)]
pub struct Predicate {
    pub agent: usize,
    pub cmp: Cmp,
    pub val: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum FormulaSymbol {
    True,
    False,
    Pred(Predicate),
    Or,
    And,
    Eventually(Ivl),
    Until(Ivl),
}

#[derive(Clone)]
pub struct FormulaNode {
    pub symb: FormulaSymbol,
    pub left: Option<Box<FormulaNode>>,
    pub right: Option<Box<FormulaNode>>,
}

#[derive(Clone, Copy)]
enum RootType {
    Left,
    Right,
}

fn valid_ivl(ivl: &Ivl) -> bool {
    ivl.lb >= Pt::from(0.)
        && ivl.lb <= ivl.ub
        && !matches!(ivl.lb, Pt::Minus(_))
        && !matches!(ivl.ub, Pt::Plus(_))
}

pub fn valid_distl_ivl(ivl: &Ivl) -> bool {
    use Pt::Exactly;
    valid_ivl(ivl) && matches!((ivl.lb, ivl.ub), (Exactly(_), Exactly(_)))
}

pub fn valid_distl_atom(formula: FormulaNode) -> bool {
    use FormulaSymbol::*;
    let msg = "Option is None";
    let no_children = |phi: &FormulaNode| phi.left.is_none() && phi.right.is_none();
    let two_children = |phi: &FormulaNode| phi.left.is_some() && phi.right.is_some();

    match formula.symb {
        True | False => no_children(&formula),
        Pred(Predicate {
            agent: a,
            cmp: _,
            val: _,
        }) => no_children(&formula) && 0 < a && a <= N,
        Or | And => {
            two_children(&formula)
                && valid_distl_atom(*formula.left.expect(msg))
                && valid_distl_atom(*formula.right.expect(msg))
        }
        _ => false,
    }
}

pub fn valid_distl(formula: FormulaNode) -> bool {
    use FormulaSymbol::*;
    let msg = "Option is None";
    let no_children = |phi: &FormulaNode| phi.left.is_none() && phi.right.is_none();
    let one_child = |phi: &FormulaNode| phi.left.is_some() && phi.right.is_none();
    let two_children = |phi: &FormulaNode| phi.left.is_some() && phi.right.is_some();

    match formula.symb {
        True | False => no_children(&formula),
        Pred(Predicate {
            agent: a,
            cmp: _,
            val: _,
        }) => no_children(&formula) && 0 < a && a <= N,
        Or => {
            if !two_children(&formula) {
                return false;
            }
            let left = *formula.left.expect(msg);
            let right = *formula.right.expect(msg);

            valid_distl(left) && valid_distl(right)
        }
        And => {
            if !two_children(&formula) {
                return false;
            }
            let left = *formula.left.expect(msg);
            let right = *formula.right.expect(msg);

            valid_distl(left) && valid_distl_atom(right)
        }
        Eventually(ivl) => {
            one_child(&formula) && valid_distl_ivl(&ivl) && valid_distl(*formula.left.expect(msg))
        }
        Until(ivl) => {
            if !two_children(&formula) {
                return false;
            }
            let left = *formula.left.expect(msg);
            let right = *formula.right.expect(msg);

            valid_distl_ivl(&ivl) && valid_distl_atom(left) && valid_distl(right)
        }
    }
}

fn roots(sig: &[(f64, f64)]) -> Vec<(f64, RootType)> {
    sig.array_windows()
        .filter_map(|&[(t1, x1), (t2, x2)]| match (x1 >= 0., x2 >= 0.) {
            (false, true) => Some((-x1 * (t2 - t1) / (x2 - x1) + t1, RootType::Left)),
            (true, false) => Some((-x1 * (t2 - t1) / (x2 - x1) + t1, RootType::Right)),
            _ => None,
        })
        .collect()
}

/// Get intervals that the signal is nonnegative.
///
/// Assumes piecewise-linear interpolation.
/// `no_zero` produces open intervals.
fn get_nonneg_ivls(sig: &[(f64, f64)], no_zero: bool) -> Vec<Ivl> {
    let sigroots = roots(sig);
    let roots_with_ends = {
        // If the ends of the signal are nonnegative then we need to complete our intervals
        let mut newvec = Vec::new();
        let firstval = sig.first().expect("the signal is empty");
        let lastval = sig.last().expect("the signal is empty");
        // The beginning of the signal is nonnegative
        if firstval.1 >= 0. {
            newvec.push((firstval.0, RootType::Left));
        }
        newvec.extend(sigroots);
        // The end of the signal is nonnegative
        if lastval.1 >= 0. {
            newvec.push((lastval.0, RootType::Right));
        }

        newvec
    };
    let (pairs, []) = roots_with_ends.as_chunks() else {
        panic!("some interval is missing an end")
    };
    pairs
        .iter()
        .filter_map(|&[(t1, l), (t2, r)]| {
            if !matches!((l, r), (RootType::Left, RootType::Right)) {
                panic!("intervals are not paired up correctly");
            }
            if no_zero {
                if t1 == t2 {
                    None
                } else {
                    Some(Ivl::new(t1, t2, false, false))
                }
            } else {
                Some(Ivl::new(t1, t2, true, true))
            }
        })
        .collect()
}

/// Get intervals that the predicate holds true on the signal.
/// Assumes piecewise-linear interpolation.
fn get_pred_ivls(sig: &[(f64, f64)], pred: Predicate) -> Vec<Ivl> {
    // Shift the signal values by the predicate value, so that we can just check against 0
    match pred.cmp {
        Cmp::Gte => get_nonneg_ivls(
            &sig.iter()
                .map(|(t, x)| (*t, x - pred.val))
                .collect::<Vec<_>>(),
            false,
        ),
        Cmp::Lt => get_nonneg_ivls(
            &sig.iter()
                .map(|(t, x)| (*t, pred.val - x))
                .collect::<Vec<_>>(),
            true,
        ),
    }
}

pub fn compute(dsig: &[Vec<(f64, f64)>; N], formula: FormulaNode) -> Vec<ConnectedBoxes> {
    use FormulaSymbol::*;
    let msg = "invalid formula: leaf node not an atom";

    match formula.symb {
        True => vec![ConnectedBoxes::new(SDBox::new_inf())],
        False => vec![],
        Pred(p) => {
            let ivls = get_pred_ivls(&dsig[p.agent - 1], p);

            // None of these should be connected to each other
            ivls.iter()
                .map(|ivl| ConnectedBoxes::new(SDBox::new_pred(p.agent, *ivl)))
                .collect()
        }
        Or => union(
            compute(dsig, *formula.left.expect(msg)),
            compute(dsig, *formula.right.expect(msg)),
        ),
        And => intersection(
            compute(dsig, *formula.left.expect(msg)),
            compute(dsig, *formula.right.expect(msg)),
        ),
        Eventually(ivl) => shift(compute(dsig, *formula.left.expect(msg)), ivl),
        Until(ivl) => until(
            compute(dsig, *formula.left.expect(msg)),
            compute(dsig, *formula.right.expect(msg)),
            ivl,
        ),
    }
}

/// Merge a collection of ConnectedBoxes so that they are all disjoint from each other.
fn merge_cbs(cbs: Vec<ConnectedBoxes>) -> Vec<ConnectedBoxes> {
    // Base case
    if cbs.len() <= 1 {
        return cbs;
    }

    // Split in half, then merge
    let mut left = cbs;
    let right = left.split_off(left.len() / 2);
    let grouped_left = merge_cbs(left);
    let grouped_right = merge_cbs(right);

    union(grouped_left, grouped_right)
}

fn union(left: Vec<ConnectedBoxes>, right: Vec<ConnectedBoxes>) -> Vec<ConnectedBoxes> {
    // Base cases
    if left.is_empty() {
        return right;
    } else if right.is_empty() {
        return left;
    }

    // Build the adjacency list
    let mut adjacency = HashMap::new();
    for ((i, c1), (j, c2)) in iproduct!(left.iter().enumerate(), right.iter().enumerate()) {
        // c1 and c2 are connected, need to add their indices i and j to the adjacency list
        if c1.connected(c2) {
            // Insert a new value into the key's set, creating a new set if
            // the key doesn't already exist
            adjacency
                .entry((0, i))
                .or_insert_with(HashSet::new)
                .insert((1, j));
            adjacency
                .entry((1, j))
                .or_insert_with(HashSet::new)
                .insert((0, i));
        }
    }

    // Now we run DFS to get all groupings from the adjacency list (any
    // search would work, since we just need all connected elements)

    let mut groupings = Vec::new();
    // Get an iterator of all the ConnectedBoxes (both `left` and `right`)
    let cb_indices = (0..left.len())
        .map(|i| (0, i))
        .chain((0..right.len()).map(|j| (1, j)));
    let mut visited = HashSet::new();
    // Combine our input sets into an array that can be accessed based on 0 or 1
    let sets = [left, right];

    for cb_index in cb_indices {
        if !visited.contains(&cb_index) {
            let mut curr_grouping = Vec::new();
            let mut stack = vec![cb_index];

            while let Some(curr) = stack.pop() {
                if !visited.contains(&curr) {
                    visited.insert(curr);
                    curr_grouping.push(sets[curr.0][curr.1].clone());
                    // Add connected CBs to the stack if they haven't been visited already
                    if let Some(connected) = adjacency.get(&curr) {
                        for next_connected in connected {
                            if !visited.contains(next_connected) {
                                stack.push(*next_connected);
                            }
                        }
                    }
                }
            }
            groupings.push(curr_grouping);
        }
    }

    // Got the groupings, now we need to merge each grouping into a single ConnectedBoxes
    groupings
        .into_iter()
        .map(|g| ConnectedBoxes::multiunion_unchecked(g).expect("grouping is empty"))
        .collect()
}

fn intersection<T>(left: T, right: T) -> Vec<ConnectedBoxes>
where
    T: IntoIterator<Item = ConnectedBoxes, IntoIter: Clone>,
{
    iproduct!(left, right)
        .map(|(l, r)| ConnectedBoxes::intersection(vec![l, r]))
        .flatten()
        .flatten()
        .collect()
}

fn shift(cbs: Vec<ConnectedBoxes>, ivl: Ivl) -> Vec<ConnectedBoxes> {
    // Shift all ConnectedBoxes and throw out the ones that are entirely shifted past 0
    merge_cbs(cbs.into_iter().filter_map(|cb| cb.shift(ivl)).collect())
}

fn until<T>(left: T, right: T, ivl: Ivl) -> Vec<ConnectedBoxes>
where
    T: IntoIterator<Item = ConnectedBoxes, IntoIter: Clone>,
{
    merge_cbs(
        iproduct!(left, right)
            .map(|(l, r)| ConnectedBoxes::until(l, r, ivl))
            .flatten()
            .flatten()
            .collect(),
    )
}

#[derive(Clone, Debug)]
pub struct ConnectedBoxes {
    boxes: Vec<SDBox>,
    bbox: SDBox,
}

impl Display for ConnectedBoxes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let boxes: Vec<String> = self.boxes.iter().map(|b| b.to_string()).collect();
        write!(
            f,
            "ConnectedBoxes {{ boxes: [{}], bbox: {} }}",
            boxes.join(", "),
            self.bbox
        )
    }
}

impl ConnectedBoxes {
    fn new(initialbox: SDBox) -> Self {
        ConnectedBoxes {
            boxes: vec![initialbox.clone()],
            bbox: initialbox,
        }
    }

    fn connected(&self, other: &Self) -> bool {
        // If bounding boxes are overlapping or touching
        if self.bbox.connected(&other.bbox) {
            for (b1, b2) in iproduct!(&self.boxes, &other.boxes) {
                if b1.connected(b2) {
                    return true;
                }
            }
        }

        // None of the individual box pairs are connected, so the two sets aren't connected
        false
    }

    /// Union of connected boxes.
    ///
    /// The returned vector has size 1 if the two are connected, and size 2 if they are not.
    fn _union(&self, other: &Self) -> Vec<Self> {
        if self.connected(other) {
            // Connected, so merge
            let mut new_boxes = self.boxes.clone();
            new_boxes.extend(other.boxes.clone());
            vec![ConnectedBoxes {
                boxes: new_boxes,
                bbox: SDBox::bbox(&[self.bbox.clone(), other.bbox.clone()])
                    .expect("no bboxes provided"),
            }]
        } else {
            // Not connected, so just return the two separately
            vec![self.clone(), other.clone()]
        }
    }

    /// Takes the union of several ConnectedBoxes into a single ConnectedBoxes,
    /// without checking that they are all connected.
    ///
    /// This is a more efficient operation than the standard union method if we know everything is connected.
    /// Returns None if the input is empty.
    fn multiunion_unchecked(cbs: Vec<Self>) -> Option<Self> {
        if cbs.is_empty() {
            return None;
        }
        let (nested_boxes, bboxes): (Vec<_>, Vec<_>) =
            cbs.into_iter().map(|cb| (cb.boxes, cb.bbox)).unzip();
        let boxes = nested_boxes.into_iter().flatten().collect();
        let bbox = SDBox::bbox(&bboxes).expect("no bboxes provided");
        Some(ConnectedBoxes { boxes, bbox })
    }

    /// Gets the intersection of multiple ConnectedBoxes and connects the pieces into new ConnectedBoxes as necessary.
    fn intersection<T: IntoIterator<Item = Self>>(cbs: T) -> Option<Vec<Self>> {
        let newboxes = Self::intersection_boxes(cbs)?;
        Some(merge_cbs(
            newboxes.into_iter().map(ConnectedBoxes::from).collect(),
        ))
    }

    /// Gets the intersection of multiple ConnectedBoxes and leaves the result as independent boxes,
    /// even if they're connected.
    fn intersection_boxes<T: IntoIterator<Item = Self>>(cbs: T) -> Option<Vec<SDBox>> {
        let (cbs_bboxes, cbs_boxes): (Vec<_>, Vec<_>) =
            cbs.into_iter().map(|cb| (cb.bbox, cb.boxes)).unzip();

        // Quick check if the bounding boxes overlap. If not, there's no intersection.
        // Also returns if `cbs` was empty.
        SDBox::intersection(&cbs_bboxes)?;

        // We want a collection that doesn't assume its groups of boxes are connected
        let grouped_boxes = cbs_boxes;

        // Do things pairwise because we can probably significantly slim down the combos of boxes we consider
        let new_boxes = grouped_boxes
            .into_iter()
            .reduce(|acc, boxes| {
                iproduct!(acc, boxes)
                    .filter_map(|(b1, b2)| SDBox::intersection(&[b1, b2]))
                    .collect()
            })
            .expect("cbs is empty");

        Some(new_boxes).filter(|vec_b| !vec_b.is_empty())
    }

    /// Shifts `Self` by the provided interval.
    ///
    /// Returns None if it entirely shifts past 0.
    fn shift(self, ivl: Ivl) -> Option<Self> {
        // Check the bbox first, then create the CB, then double-check that there's
        // at least one box in the CB
        self.bbox
            .shift_bbox(ivl)
            .and_then(SDBox::shorten)
            .map(|bbox| ConnectedBoxes {
                boxes: self
                    .boxes
                    .into_iter()
                    .flat_map(|b| b.shift(ivl).into_iter().filter_map(SDBox::shorten))
                    .collect(),
                bbox,
            })
            .filter(|cb| !cb.boxes.is_empty())
    }

    /// Computes `left` Until_{ivl} `right`.
    ///
    /// Returns None if `left` and `right` don't intersect or there's no Until results.
    fn until(left: ConnectedBoxes, right: ConnectedBoxes, ivl: Ivl) -> Option<Vec<Self>> {
        // Get intersection of left and right
        let mut intersections = ConnectedBoxes::intersection_boxes([left.clone(), right.clone()])?;
        intersections.sort_by(|a, b| {
            a.sides[0]
                .lb
                .partial_cmp(&b.sides[0].lb)
                .expect("f64 is NaN")
        });
        // Get groupings of N+1 offsets
        let intersection_groups: Vec<_> = intersections
            .chunk_by(|a, b| a.sides[0] == b.sides[0])
            .map(|g| (g[0].sides[0] - ivl, g.to_vec()))
            .collect();
        let mut results = Vec::new();
        // For each grouping:
        for (sat_bound, grouping) in intersection_groups {
            if !valid_ivl(&sat_bound) {
                // These boxes don't have any satisfying region that's >= 0
                continue;
            }
            // Add each box in grouping to queue, and visited
            let curr_grouping = grouping.clone();
            let mut visited = HashSet::new();
            for b in curr_grouping {
                visited.insert(b);
            }
            let mut queue = VecDeque::from(grouping);

            // While queue is not empty, pop box b off the queue and:
            while let Some(mut b) = queue.pop_front() {
                // Check if b is in the N+1 offset for this grouping
                if b.sides[0].lb <= sat_bound.ub {
                    if b.sides[0].ub < sat_bound.lb {
                        // Past the offset; this is a dead end. Continue to the next iteration of the while loop
                        continue;
                    } else {
                        // In the offset, add b to results
                        let mut b_result = b.clone();
                        b_result.sides[0] = Ivl::intersection(&[b_result.sides[0], sat_bound])
                            .expect("box not in region");
                        results.push(ConnectedBoxes::new(b_result));
                    }
                }
                // Get all neighbors of b (connected boxes)
                let neighbors = b.get_connected(left.boxes.clone());
                // Make b a lower closure
                for side in b.sides.iter_mut() {
                    side.lb = Pt::from(0.);
                }
                // Intersect each neighbor with the lower closure of b;
                // this finds the neighboring boxes that are "left and down"
                // from b.
                for neighbor in neighbors {
                    let b_closure = b.clone();
                    // Get the intersection. We also apply our intersection to `SDBox::open_top()` since
                    // all upper boundaries of the box can't have a retiming passing along them that hasn't
                    // already been found (when evaluating b)
                    if let Some(intersected_neighbor) =
                        SDBox::intersection(&[b_closure, neighbor]).and_then(|x| x.open_top())
                    {
                        // Check that the intersected neighbor hasn't been visited yet
                        if !visited.contains(&intersected_neighbor) {
                            // Add intersected neighbor to visited and queue
                            visited.insert(intersected_neighbor.clone());
                            queue.push_back(intersected_neighbor);
                        }
                    }
                }
            }
        }

        // Return results (if empty, return None)
        Some(merge_cbs(results)).filter(|x| !x.is_empty())
    }
}

impl From<SDBox> for ConnectedBoxes {
    fn from(value: SDBox) -> Self {
        ConnectedBoxes {
            boxes: vec![value.clone()],
            bbox: value,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct SDBox {
    sides: [Ivl; N + 1],
}

impl Display for SDBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bx [{}]",
            self.sides.map(|ivl| ivl.to_string()).join(", ")
        )
    }
}

impl SDBox {
    /// Returns an infinitely large box.
    fn new_inf() -> Self {
        let sides = [Ivl::new_inf(); N + 1];
        SDBox { sides }
    }

    /// Returns a box from a predicate.
    fn new_pred(agent: usize, ivl: Ivl) -> Self {
        let mut sides = [Ivl::new_inf(); N + 1];

        for (i, side) in sides.iter_mut().enumerate() {
            let diff = if i == agent {
                0.
            } else if i == 0 {
                EPS
            } else {
                EPS * 2.
            };

            // Clamp the lower bound at 0
            side.lb = (ivl.lb - diff).max(Pt::from(0.));
            side.ub = ivl.ub + diff;
        }

        SDBox { sides }
    }

    /// Finds the bounding box of multiple provided boxes.
    ///
    /// Returns None if no boxes are provided.
    fn bbox(boxes: &[Self]) -> Option<Self> {
        // Check that some boxes were provided
        if boxes.is_empty() {
            return None;
        }

        let mut sides = [Ivl::new_inf(); N + 1];

        for (i, side) in sides.iter_mut().enumerate() {
            *side = Ivl::span(&boxes.iter().map(|b| b.sides[i]).collect::<Vec<_>>())
                .expect("no boxes provided");
        }

        Some(SDBox { sides })
    }

    /// Checks if two boxes are connected to each other.
    fn connected(&self, other: &Self) -> bool {
        for i in 0..=N {
            if !self.sides[i].connected(&other.sides[i]) {
                return false;
            }
        }

        true
    }

    /// Returns all boxes in `other` that are connected with `self`.
    fn get_connected(&self, other: Vec<Self>) -> Vec<Self> {
        other.into_iter().filter(|b| self.connected(b)).collect()
    }

    /// Gets the intersection of multiple boxes.
    ///
    /// Returns None if the intersection is empty.
    fn intersection(boxes: &[Self]) -> Option<Self> {
        let mut sides = [Ivl::new_inf(); N + 1];

        for (i, side) in sides.iter_mut().enumerate() {
            // If any one side returns None (no intersection), the whole function returns None
            *side = Ivl::intersection(&boxes.iter().map(|b| b.sides[i]).collect::<Vec<_>>())?;
        }

        Some(SDBox { sides })
    }

    /// Shifts an SDBox by a provided interval along the reference dimension.
    ///
    /// Panics if the interval is not closed.
    fn shift(self, ivl: Ivl) -> Vec<Self> {
        let mut sides = self.sides;
        let old_lb: Pt = sides[0].lb;

        let (ivl_lb, ivl_ub) = match (ivl.lb, ivl.ub) {
            (Pt::Exactly(lb), Pt::Exactly(ub)) => (lb, ub),
            (_, _) => panic!("`ivl` is not closed"),
        };
        sides[0].lb = (sides[0].lb - ivl_ub).max(Pt::from(0.));
        sides[0].ub = sides[0].ub - ivl_lb;

        // If the right side goes past 0, we've lost the box
        if sides[0].ub < Pt::from(0.) {
            return vec![];
        }

        let mut boxes = Vec::new();

        if ivl_lb == 0. {
            // Only keep the extra box if we're not already bumped up against 0
            if old_lb != Pt::from(0.) {
                let mut sides2 = sides;
                sides2[0].ub = old_lb;
                sides[0].lb = old_lb;
                // The extra box has open upper boundaries
                for side in sides2[1..].iter_mut() {
                    side.lb = Pt::from(0.);
                    side.ub = match side.ub {
                        Pt::Exactly(x) | Pt::Minus(x) => Pt::Minus(x),
                        Pt::Plus(_) => panic!("Not a valid interval"),
                    };
                }
                boxes.push(sides2);
            }
        } else {
            // When the interval doesn't contain 0, upper boundaries are open
            for side in sides[1..].iter_mut() {
                side.ub = match side.ub {
                    Pt::Exactly(x) | Pt::Minus(x) => Pt::Minus(x),
                    Pt::Plus(_) => panic!("Not a valid interval"),
                };
            }
        }

        for side in sides[1..].iter_mut() {
            side.lb = Pt::from(0.);
        }
        boxes.push(sides);

        boxes.into_iter().map(|s| SDBox { sides: s }).collect()
    }

    /// Shifts a bounding box by a provided interval along the reference dimension.
    ///
    /// Returns None if the box moves past the 0 boundary.
    /// Panics if the interval is not closed.
    fn shift_bbox(self, ivl: Ivl) -> Option<Self> {
        let mut sides = self.sides;

        let (ivl_lb, ivl_ub) = match (ivl.lb, ivl.ub) {
            (Pt::Exactly(lb), Pt::Exactly(ub)) => (lb, ub),
            (_, _) => panic!("`ivl` is not closed"),
        };
        sides[0].lb = (sides[0].lb - ivl_ub).max(Pt::from(0.));
        sides[0].ub = sides[0].ub - ivl_lb;
        // If the right side goes past 0, we've lost the box
        if sides[0].ub < Pt::from(0.) {
            return None;
        }

        // If we're shifting by more than 0 on the lower bound of `ivl`,
        // then the upper bounds of all dimensions besides the reference
        // must be adjusted to be open
        if ivl_lb != 0. {
            for side in sides[1..].iter_mut() {
                side.ub = match side.ub {
                    Pt::Exactly(x) | Pt::Minus(x) => Pt::Minus(x),
                    Pt::Plus(_) => panic!("Not a valid interval"),
                };
            }
        }

        for side in sides[1..].iter_mut() {
            side.lb = Pt::from(0.);
        }

        Some(SDBox { sides })
    }

    /// Shrinks a box so that it minimally fits the eps constraints; all other
    /// space in the box is unnecessary.
    fn shorten(self) -> Option<Self> {
        // The lower bound of the reference (0th) dimension
        let ref_lb = self.sides[1..]
            .iter()
            .fold(self.sides[0].lb, |acc, ivl| acc.max(ivl.lb - EPS));
        // The upper bound of the reference (0th) dimension
        let ref_ub = self.sides[1..]
            .iter()
            .fold(self.sides[0].ub, |acc, ivl| acc.min(ivl.ub + EPS));

        if !valid_ivl(&Ivl {
            lb: ref_lb,
            ub: ref_ub,
        }) {
            // The entire box exists outside of the eps bounds
            return None;
        }

        let mut sides = self.sides;
        for side in sides.iter_mut() {
            side.lb = side.lb.max(ref_lb - EPS);
            side.ub = side.ub.min(ref_ub + EPS);
        }

        Some(SDBox { sides })
    }

    /// Make all upper bounds of the box open.
    ///
    /// Returns None if that makes it a non-valid box.
    fn open_top(self) -> Option<Self> {
        let mut sides = self.sides;
        for side in sides.iter_mut() {
            side.ub = Pt::Minus(side.ub.get_val());
            if !valid_ivl(&side) {
                return None;
            }
        }

        Some(SDBox { sides })
    }

    /// Checks that two `SDBox`s are very close together.
    ///
    /// Considered close if any given side is less than `eps` apart. Useful for testing.
    pub fn rounded_eq(&self, other: &Self, eps: f64) -> bool {
        self.sides
            .iter()
            .zip(other.sides)
            .all(|(ivl1, ivl2)| ivl1.rounded_eq(&ivl2, eps))
    }
}

/// Represents either a number or the limit as we approach the number.
///
/// I.e., either x, lim -> x-, or lim -> x+.
/// This is useful for working with interval bounds, particularly
/// when we need to represent a point using an `SDBox`.
///
/// We say that Minus(NaN) == Minus(NaN), Plus(NaN) == Plus(NaN), and
/// Exactly(NaN) == Exactly(NaN), even though NaN != NaN.
/// We do this for hashing purposes.
#[derive(Clone, Copy, Debug)]
pub enum Pt {
    Minus(f64),
    Plus(f64),
    Exactly(f64),
}

impl PartialOrd for Pt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pt {
    fn cmp(&self, other: &Self) -> Ordering {
        // For our purposes, we say that NaN == NaN
        const PRECISION_SHIFT: f64 = 10_i32.pow(PRECISION) as f64;
        let x = (self.get_val() * PRECISION_SHIFT) as i32;
        let y = (other.get_val() * PRECISION_SHIFT) as i32;

        if x != y {
            // If the numbers aren't equal, follow their ordering
            x.cmp(&y)
        } else {
            // If the numbers are the same, we have some additional ordering to do
            match (self, other) {
                (Self::Minus(_), Self::Minus(_))
                | (Self::Plus(_), Self::Plus(_))
                | (Self::Exactly(_), Self::Exactly(_)) => Ordering::Equal,
                (Self::Minus(_), _) | (_, Self::Plus(_)) => Ordering::Less,
                (Self::Plus(_), _) | (_, Self::Minus(_)) => Ordering::Greater,
            }
        }
    }
}

impl PartialEq for Pt {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Equal))
    }
}

impl Eq for Pt {}

impl Add<f64> for Pt {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        use Pt::*;
        match self {
            Minus(x) => Minus(x + rhs),
            Plus(x) => Plus(x + rhs),
            Exactly(x) => Exactly(x + rhs),
        }
    }
}

impl Sub<f64> for Pt {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        use Pt::*;
        match self {
            Minus(x) => Minus(x - rhs),
            Plus(x) => Plus(x - rhs),
            Exactly(x) => Exactly(x - rhs),
        }
    }
}

impl Sub<Pt> for Pt {
    type Output = Self;

    fn sub(self, rhs: Pt) -> Self::Output {
        use Pt::*;
        match (self, rhs) {
            (Minus(x), Minus(y)) => Exactly(x - y),
            (Minus(x), Plus(y)) => Minus(x - y),
            (Minus(x), Exactly(y)) => Minus(x - y),
            (Plus(x), Minus(y)) => Plus(x - y),
            (Plus(x), Plus(y)) => Exactly(x - y),
            (Plus(x), Exactly(y)) => Plus(x - y),
            (Exactly(x), Minus(y)) => Plus(x - y),
            (Exactly(x), Plus(y)) => Minus(x - y),
            (Exactly(x), Exactly(y)) => Exactly(x - y),
        }
    }
}

impl From<f64> for Pt {
    fn from(value: f64) -> Self {
        Pt::Exactly(value)
    }
}

impl Display for Pt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Note: Since we have that NaN == NaN for Pt, we need to be sure that
        // printing NaN is the same every time (since we're using Display for hashing).
        match self {
            Self::Minus(x) => write!(f, "{}-", x),
            Self::Plus(x) => write!(f, "{}+", x),
            Self::Exactly(x) => write!(f, "{}=", x),
        }
    }
}

impl Hash for Pt {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        const PRECISION_SHIFT: f64 = 10_i32.pow(PRECISION) as f64;
        ((self.get_val() * PRECISION_SHIFT) as i32).hash(state);
    }
}

impl Pt {
    /// Min function.
    ///
    /// If equal, returns `self`.
    fn min(self, other: Self) -> Self {
        if self <= other { self } else { other }
    }

    /// Max function.
    ///
    /// If equal, returns `self`.
    fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    /// Retrieves the value, regardless of the variant it's in.
    pub const fn get_val(&self) -> f64 {
        use Pt::*;
        match self {
            Minus(x) | Plus(x) | Exactly(x) => *x,
        }
    }

    /// Checks if two values are next to each other.
    ///
    /// Includes if they're the same point.
    fn adjacent(self, other: Self) -> bool {
        // Two different values. Not adjacent.
        if self.get_val() != other.get_val() {
            return false;
        }

        use Pt::*;
        match (self, other) {
            (Minus(_), Plus(_)) | (Plus(_), Minus(_)) => false,
            (_, _) => true,
        }
    }

    /// Checks that two `Pt`s are very close together.
    ///
    /// Considered close if they are less than `eps` apart. Useful for testing.
    pub fn rounded_eq(&self, other: &Self, eps: f64) -> bool {
        use Pt::*;
        match (self, other) {
            (Minus(x), Minus(y)) | (Plus(x), Plus(y)) | (Exactly(x), Exactly(y)) => {
                (x - y).abs() < eps
            }
            _ => false,
        }
    }
}

/// An interval. We assume none of the interval is negative (meaning lb is always >= 0).
#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub struct Ivl {
    pub lb: Pt,
    pub ub: Pt,
}

impl Sub<Ivl> for Ivl {
    type Output = Self;

    fn sub(self, rhs: Ivl) -> Self::Output {
        // Floor only lb at 0 so that if the entire ivl is out of bounds,
        // we can find out with `valid_ivl()`
        Ivl {
            lb: (self.lb - rhs.ub).max(Pt::from(0.)),
            ub: self.ub - rhs.lb,
        }
    }
}

impl Display for Ivl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ivl{{{},{}}}", self.lb, self.ub)
    }
}

impl Ivl {
    /// Returns an infinitely large interval.
    fn new_inf() -> Self {
        // Make the upper bound closed, since most operations with an interval
        // involve leaving the boundary type the same
        Ivl {
            lb: Pt::from(0.),
            ub: Pt::from(f64::INFINITY),
        }
    }

    /// Produces a new open or closed interval from provided values.
    pub fn new(lb: f64, ub: f64, closed_lb: bool, closed_ub: bool) -> Self {
        let lb_wrapped = if closed_lb {
            Pt::Exactly(lb)
        } else {
            Pt::Plus(lb)
        };
        let ub_wrapped = if closed_ub {
            Pt::Exactly(ub)
        } else {
            Pt::Minus(ub)
        };
        Ivl {
            lb: lb_wrapped,
            ub: ub_wrapped,
        }
    }

    /// Checks if two intervals are connected to each other.
    ///
    /// This includes both a nonempty intersection as well as touching boundaries.
    fn connected(&self, other: &Self) -> bool {
        (self.lb <= other.ub && other.lb <= self.ub)
            || self.lb.adjacent(other.ub)
            || self.ub.adjacent(other.lb)
    }

    /// Gets the intersection of multiple intervals.
    ///
    /// Returns None if the intersection is empty.
    fn intersection(ivls: &[Self]) -> Option<Self> {
        // Get lower and upper bounds of the intersection. This
        // may not be a valid interval.
        let lb = ivls
            .iter()
            .map(|ivl| ivl.lb)
            .reduce(|acc, x| acc.max(x))
            .unwrap_or(Pt::from(f64::INFINITY));
        let ub = ivls
            .iter()
            .map(|ivl| ivl.ub)
            .reduce(|acc, x| acc.min(x))
            .unwrap_or(Pt::from(f64::NEG_INFINITY));

        // May not be a valid interval, so we check and return None if not
        Some(Ivl { lb, ub }).filter(valid_ivl)
    }

    /// Gets the range of values spanned by the intervals; from the lowest of the
    /// lower bounds to the highest of the upper bounds.
    ///
    /// Returns None if no intervals are provided.
    fn span(ivls: &[Self]) -> Option<Self> {
        if ivls.is_empty() {
            return None;
        }

        let lb = ivls
            .iter()
            .map(|ivl| ivl.lb)
            .reduce(|acc, x| acc.min(x))
            .expect("`ivls` is empty");
        let ub = ivls
            .iter()
            .map(|ivl| ivl.ub)
            .reduce(|acc, x| acc.max(x))
            .expect("`ivls` is empty");

        // Don't need to worry about this not being a valid interval, since that would only happen if ivls was empty
        Some(Ivl { lb, ub })
    }

    /// Checks that two `Ivl`s are very close together.
    ///
    /// Considered close if they are less than `eps` apart. Useful for testing.
    pub fn rounded_eq(&self, other: &Self, eps: f64) -> bool {
        self.lb.rounded_eq(&other.lb, eps) && self.ub.rounded_eq(&other.ub, eps)
    }
}

// #[cfg(test)]
// mod unit_tests {
//     use super::*;

//     // -------------------------------------------------------------
//     // 1. BASIC INTERVAL & POINT TESTS
//     // -------------------------------------------------------------

//     #[test]
//     fn test_pt_ordering() {
//         use Pt::*;

//         assert!(Minus(1.0) < Exactly(1.0));
//         assert!(Exactly(1.0) < Plus(1.0));
//         assert!(Minus(1.0) < Plus(1.0));

//         assert!(Plus(2.0) > Exactly(2.0));
//         assert!(Exactly(2.0) > Minus(2.0));
//     }

//     #[test]
//     fn test_ivl_new_open_closed() {
//         let a = Ivl::new(1.0, 2.0, true, true);
//         assert_eq!(a.lb, Pt::Exactly(1.0));
//         assert_eq!(a.ub, Pt::Exactly(2.0));

//         let b = Ivl::new(1.0, 2.0, false, true);
//         assert_eq!(b.lb, Pt::Plus(1.0));
//         assert_eq!(b.ub, Pt::Exactly(2.0));
//     }

//     #[test]
//     fn test_ivl_connected() {
//         let a = Ivl::new(0.0, 1.0, true, true);
//         let b = Ivl::new(1.0, 2.0, true, true);
//         assert!(a.connected(&b)); // touching counts

//         let a = Ivl::new(0.0, 1.0, true, false);
//         let b = Ivl::new(1.0, 2.0, false, true);
//         assert!(!a.connected(&b)); // open intervals on both sides shouldn't count
//     }

//     #[test]
//     fn test_valid_ivl() {
//         let good = Ivl::new(0.0, 5.0, true, true);
//         assert!(valid_ivl(&good));

//         let bad = Ivl::new(5.0, 1.0, true, true);
//         assert!(!valid_ivl(&bad));
//     }

//     // -------------------------------------------------------------
//     // 2. ROOT FINDING & SIGNAL INTERVAL EXTRACTION
//     // -------------------------------------------------------------

//     #[test]
//     fn test_roots_simple_crossing() {
//         // crosses from negative to positive at t = 0.5
//         let sig = vec![(0.0, -1.0), (1.0, 1.0)];
//         let r = roots(&sig);
//         assert_eq!(r.len(), 1);
//         assert!(matches!(r[0].1, RootType::Left));
//         assert!((r[0].0 - 0.5).abs() < 1e-6);
//     }

//     #[test]
//     fn test_get_nonneg_ivls_basic() {
//         let sig = vec![(0.0, -1.0), (1.0, 1.0), (2.0, 1.0), (3.0, -1.0)];

//         let ivls = get_nonneg_ivls(&sig, false);
//         assert_eq!(ivls.len(), 1);

//         let ivl = ivls[0];
//         assert!((ivl.lb.get_val() - 0.5).abs() < 1e-6);
//         assert!((ivl.ub.get_val() - 2.5).abs() < 1e-6);
//     }

//     #[test]
//     fn test_get_pred_ivls_gte() {
//         let sig = vec![(0.0, 0.0), (1.0, 2.0)];
//         let pred = Predicate {
//             agent: 1,
//             cmp: Cmp::Gte,
//             val: 1.0,
//         };

//         let ivls = get_pred_ivls(&sig, pred);
//         assert_eq!(ivls.len(), 1);

//         let ivl = ivls[0];
//         assert!(ivl.lb.get_val() > 0.0);
//         assert!(ivl.ub.get_val() >= 1.0);
//     }

//     // -------------------------------------------------------------
//     // 3. SDBox TESTS
//     // -------------------------------------------------------------

//     #[test]
//     fn test_sdbox_new_pred_expansion() {
//         let ivl = Ivl::new(1.0, 2.0, true, true);
//         let b = SDBox::new_pred(1, ivl);

//         // agent dimension should match exactly
//         assert_eq!(b.sides[1].lb.get_val(), 1.0);
//         assert_eq!(b.sides[1].ub.get_val(), 2.0);

//         // reference dimension widened by EPS
//         assert!((b.sides[0].lb.get_val() - (1.0 - EPS)).abs() < 1e-6);
//         assert!((b.sides[0].ub.get_val() - (2.0 + EPS)).abs() < 1e-6);
//     }

//     #[test]
//     fn test_sdbox_connected() {
//         let a = SDBox::new_pred(1, Ivl::new(0.0, 1.0, true, true));
//         let b = SDBox::new_pred(1, Ivl::new(1.0, 2.0, true, true));
//         assert!(a.connected(&b)); // connected on an edge

//         let a = SDBox::new_pred(1, Ivl::new(0.0, 1.0, true, true));
//         let b = SDBox::new_pred(2, Ivl::new(1.0 + 2. * EPS, 2.0, true, true));
//         assert!(a.connected(&b)); // touching a corner

//         let a = SDBox::new_pred(1, Ivl::new(0.0, 1.0, true, false));
//         let b = SDBox::new_pred(2, Ivl::new(1.0 + 2. * EPS, 2.0, false, true));
//         assert!(!a.connected(&b)); // corner isn't touching
//     }

//     #[test]
//     fn test_sdbox_intersection() {
//         let a = SDBox::new_pred(1, Ivl::new(0.0, 2.0, true, true));
//         let b = SDBox::new_pred(1, Ivl::new(1.0, 3.0, true, true));

//         let intersected = SDBox::intersection(&[a, b]).unwrap();
//         assert!((intersected.sides[1].lb.get_val() - 1.0).abs() < 1e-6);
//         assert!((intersected.sides[1].ub.get_val() - 2.0).abs() < 1e-6);
//     }

//     // -------------------------------------------------------------
//     // 4. CONNECTEDBOXES TESTS
//     // -------------------------------------------------------------

//     #[test]
//     fn test_connectedboxes_union_connected() {
//         let a = ConnectedBoxes::new(SDBox::new_pred(1, Ivl::new(0.0, 1.0, true, true)));
//         let b = ConnectedBoxes::new(SDBox::new_pred(1, Ivl::new(1.0, 2.0, false, true)));

//         let u = a.union(&b);
//         assert_eq!(u.len(), 1); // merged
//     }

//     #[test]
//     fn test_connectedboxes_union_disconnected() {
//         let a = ConnectedBoxes::new(SDBox::new_pred(1, Ivl::new(0.0, 1.0, true, true)));
//         let b = ConnectedBoxes::new(SDBox::new_pred(1, Ivl::new(5.0, 6.0, true, true)));

//         let u = a.union(&b);
//         assert_eq!(u.len(), 2);
//     }

//     // -------------------------------------------------------------
//     // 5. FORMULA VALIDATION TESTS
//     // -------------------------------------------------------------

//     #[test]
//     fn test_valid_distl_atom_pred() {
//         let f = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 1.0,
//             }),
//             left: None,
//             right: None,
//         };
//         assert!(valid_distl_atom(f));
//     }

//     #[test]
//     fn test_valid_distl_and_right_must_be_atom() {
//         let left = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 1.0,
//             }),
//             left: None,
//             right: None,
//         };
//         let right = FormulaNode {
//             symb: FormulaSymbol::Eventually(Ivl::new(0.0, 1.0, true, true)),
//             left: Some(Box::new(left.clone())),
//             right: None,
//         };
//         let f = FormulaNode {
//             symb: FormulaSymbol::And,
//             left: Some(Box::new(left.clone())),
//             right: Some(Box::new(right.clone())),
//         };
//         assert!(!valid_distl(f)); // right is not an atom

//         let f = FormulaNode {
//             symb: FormulaSymbol::And,
//             left: Some(Box::new(right)),
//             right: Some(Box::new(left)),
//         };
//         assert!(valid_distl(f)); // atom on the left is valid
//     }

//     // -------------------------------------------------------------
//     // 6. END-TO-END COMPUTE TESTS
//     // -------------------------------------------------------------

//     #[test]
//     fn test_compute_simple_predicate() {
//         let sig = [
//             vec![(0.0, -1.0), (1.0, 2.0)], // agent 1
//             vec![(0.0, 0.0), (1.0, 0.0)],  // agent 2 (unused)
//         ];

//         let f = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let out = compute(&sig, f);
//         assert!(!out.is_empty());
//     }

//     #[test]
//     fn test_compute_or() {
//         let sig = [vec![(0.0, -1.0), (1.0, 2.0)], vec![(0.0, 1.0), (1.0, -1.0)]];

//         let p1 = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };
//         let p2 = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 2,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let f = FormulaNode {
//             symb: FormulaSymbol::Or,
//             left: Some(Box::new(p1)),
//             right: Some(Box::new(p2)),
//         };

//         let out = compute(&sig, f);
//         assert!(!out.is_empty());
//     }

//     #[test]
//     fn test_until() {
//         let new_box = |(x1, y1, z1), (x2, y2, z2)| SDBox {
//             sides: [
//                 Ivl::new(x1, x2, true, true),
//                 Ivl::new(y1, y2, true, true),
//                 Ivl::new(z1, z2, true, true),
//             ],
//         };
//         let left_boxes = vec![
//             new_box((0.21, 0.21, 0.21), (0.29, 0.26, 0.26)),
//             new_box((0.25, 0.25, 0.25), (0.27, 0.28, 0.28)),
//             new_box((0.29, 0.21, 0.21), (0.32, 0.32, 0.32)),
//             new_box((0.29, 0.32, 0.32), (0.35, 0.41, 0.41)),
//             new_box((0.35, 0.31, 0.31), (0.41, 0.39, 0.39)),
//             new_box((0.39, 0.34, 0.34), (0.47, 0.43, 0.43)),
//         ];
//         let right_boxes = vec![
//             new_box((0.20, 0.20, 0.20), (0.28, 0.29, 0.29)),
//             new_box((0.32, 0.40, 0.40), (0.38, 0.42, 0.42)),
//             new_box((0.32, 0.42, 0.42), (0.41, 0.45, 0.45)),
//             new_box((0.41, 0.42, 0.42), (0.44, 0.46, 0.46)),
//             new_box((0.43, 0.41, 0.41), (0.46, 0.50, 0.50)),
//         ];

//         let left_cbs = merge_cbs(
//             left_boxes
//                 .into_iter()
//                 .map(|b| ConnectedBoxes::from(b))
//                 .collect(),
//         );
//         let right_cbs = merge_cbs(
//             right_boxes
//                 .into_iter()
//                 .map(|b| ConnectedBoxes::from(b))
//                 .collect(),
//         );
//         assert_eq!(left_cbs.len(), 1);
//         assert_eq!(right_cbs.len(), 2);

//         let ivl1 = Ivl::new(0.08, 0.10, true, true);
//         let results1 = until(left_cbs.clone(), right_cbs.clone(), ivl1);
//         let correct1_1 = SDBox {
//             sides: [
//                 Ivl::new(0.22, 0.27, true, true),
//                 Ivl::new(0.21, 0.26, true, false),
//                 Ivl::new(0.21, 0.26, true, false),
//             ],
//         };
//         let correct1_2 = SDBox {
//             sides: [
//                 Ivl::new(0.29, 0.38, true, true),
//                 Ivl::new(0.21, 0.39, true, false),
//                 Ivl::new(0.21, 0.39, true, false),
//             ],
//         };

//         assert_eq!(results1.len(), 2);
//         assert!(results1[0].bbox.rounded_eq(&correct1_1, 1e-6));
//         assert!(results1[1].bbox.rounded_eq(&correct1_2, 1e-6));

//         let ivl2 = Ivl::new(0., 0.2, true, true);
//         let results2 = until(left_cbs.clone(), right_cbs.clone(), ivl2);
//         let correct2 = SDBox {
//             sides: [
//                 Ivl::new(0.21, 0.46, true, true),
//                 Ivl::new(0.21, 0.43, true, true),
//                 Ivl::new(0.21, 0.43, true, true),
//             ],
//         };
//         assert_eq!(results2.len(), 1);
//         assert!(results2[0].bbox.rounded_eq(&correct2, 1e-6));
//     }
// }

// #[cfg(test)]
// mod golden_tests {
//     use super::*;

//     // Helper: extract interval from a given dimension of a ConnectedBoxes
//     fn dim_ivl(cb: &ConnectedBoxes, dim: usize) -> (f64, f64) {
//         let ivl = cb.bbox.sides[dim];
//         (ivl.lb.get_val(), ivl.ub.get_val())
//     }

//     // Helper: extract the agent interval (dimension = agent index)
//     fn agent_ivl(cb: &ConnectedBoxes, agent: usize) -> (f64, f64) {
//         dim_ivl(cb, agent)
//     }

//     // Helper: extract reference dimension interval (dimension = 0)
//     fn ref_ivl(cb: &ConnectedBoxes) -> (f64, f64) {
//         dim_ivl(cb, 0)
//     }

//     // -------------------------------------------------------------
//     // GOLDEN 1: Simple predicate
//     // Agent 1 >= 0 on [1,2]
//     // -------------------------------------------------------------
//     #[test]
//     fn golden_predicate_simple() {
//         let sig = [
//             vec![(0.0, -1.0), (1.0, 0.0), (2.0, 3.0)], // agent 1
//             vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],  // agent 2 unused
//         ];

//         let f = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let out = compute(&sig, f);
//         assert_eq!(out.len(), 1);

//         // Agent dimension must match the logical interval [1,2]
//         let (lb, ub) = agent_ivl(&out[0], 1);
//         assert!((lb - 1.0).abs() < 1e-6);
//         assert!((ub - 2.0).abs() < 1e-6);

//         // Reference dimension widened by EPS
//         let (rlb, rub) = ref_ivl(&out[0]);
//         assert!(rlb <= 1.0);
//         assert!(rub >= 2.0);
//     }

//     // -------------------------------------------------------------
//     // GOLDEN 2: OR merges into one ConnectedBoxes
//     // Agent 1 >= 0 on [0,1]
//     // Agent 2 >= 0 on [1,2]
//     // Touch at 1 → connected → one ConnectedBoxes
//     // -------------------------------------------------------------
//     #[test]
//     fn golden_or_connected() {
//         let sig = [
//             vec![(0.0, 1.0), (1.0, 0.0), (2.0, -1.0)], // agent 1: [0,1]
//             vec![(0.0, -1.0), (1.0, 0.0), (2.0, 1.0)], // agent 2: [1,2]
//         ];

//         let p1 = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };
//         let p2 = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 2,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let f = FormulaNode {
//             symb: FormulaSymbol::Or,
//             left: Some(Box::new(p1)),
//             right: Some(Box::new(p2)),
//         };

//         let out = compute(&sig, f);

//         assert_eq!(out.len(), 1);

//         // Reference dimension must cover [0,2] (plus EPS)
//         let (rlb, rub) = ref_ivl(&out[0]);
//         assert!(rlb <= 0.0);
//         assert!(rub >= 2.0);
//     }

//     // -------------------------------------------------------------
//     // GOLDEN 3: AND intersection
//     // Agent 1 >= 0 on [1,2]
//     // Agent 2 >= 0 on [0,1]
//     // Intersection = [1-eps, 1+eps] (on dimension 0)
//     // -------------------------------------------------------------
//     #[test]
//     fn golden_and_intersection() {
//         let sig = [
//             vec![(0.0, -1.0), (1.0, 0.0), (2.0, 3.0)], // agent 1: [1,2]
//             vec![(0.0, 1.0), (1.0, 0.0), (2.0, -1.0)], // agent 2: [0,1]
//         ];

//         let p1 = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };
//         let p2 = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 2,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let f = FormulaNode {
//             symb: FormulaSymbol::And,
//             left: Some(Box::new(p1)),
//             right: Some(Box::new(p2)),
//         };

//         let out = compute(&sig, f);

//         assert_eq!(out.len(), 1);

//         // Intersection is t = [1-eps, 1+eps]
//         let (lb, ub) = ref_ivl(&out[0]);

//         assert!((lb - (1.0 - EPS)).abs() < 1e-6);
//         assert!((ub - (1.0 + EPS)).abs() < 1e-6);
//     }

//     // -------------------------------------------------------------
//     // GOLDEN 4: Eventually shift
//     // Predicate holds on [1,2]
//     // Eventually_[0,1] shifts to [0,2]
//     // -------------------------------------------------------------
//     #[test]
//     fn golden_eventually_shift() {
//         let sig = [
//             vec![(0.0, -1.0), (1.0, 0.0), (2.0, 3.0)], // agent 1: [1,2]
//             vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],  // agent 2 unused
//         ];

//         let pred = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let f = FormulaNode {
//             symb: FormulaSymbol::Eventually(Ivl::new(0.0, 1.0, true, true)),
//             left: Some(Box::new(pred)),
//             right: None,
//         };

//         let out = compute(&sig, f);

//         assert_eq!(out.len(), 1);

//         // Reference dimension must cover [0,2] (plus EPS)
//         let (lb, ub) = ref_ivl(&out[0]);
//         assert!((lb - 0.0).abs() < 1e-6);
//         assert!((ub - (2.0 + EPS)).abs() < 1e-6);
//     }

//     // -------------------------------------------------------------
//     // GOLDEN 5: Until
//     // left: agent 1 >= 0 on [0,2]
//     // right: agent 2 >= 0 at t = 1
//     // interval [0,1]
//     // Until holds on [0,1]
//     // -------------------------------------------------------------
//     #[test]
//     fn golden_until_simple() {
//         let sig = [
//             vec![(0.0, 1.0), (2.0, 1.0)],               // agent 1: always >=0
//             vec![(0.0, -1.0), (1.0, 0.0), (2.0, -1.0)], // agent 2: only at t=1
//         ];

//         let left = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 1,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };
//         let right = FormulaNode {
//             symb: FormulaSymbol::Pred(Predicate {
//                 agent: 2,
//                 cmp: Cmp::Gte,
//                 val: 0.0,
//             }),
//             left: None,
//             right: None,
//         };

//         let f = FormulaNode {
//             symb: FormulaSymbol::Until(Ivl::new(0.0, 1.0, true, true)),
//             left: Some(Box::new(left)),
//             right: Some(Box::new(right)),
//         };

//         let out = compute(&sig, f);

//         assert_eq!(out.len(), 1);

//         // Reference dimension must cover [0,1] (plus EPS)
//         let (lb, ub) = ref_ivl(&out[0]);
//         println!("{}", lb);
//         assert!((lb - 0.0).abs() < 1e-6);
//         assert!((ub - (1.0 + EPS)).abs() < 1e-6);
//     }
// }
