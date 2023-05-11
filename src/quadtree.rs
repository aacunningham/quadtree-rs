pub struct Quadtree<T> {
    inner: Node<T>,
}
impl<T> Quadtree<T>
where
    T: Default + Copy + PartialEq,
{
    pub fn new<C>(lower_left_bound: C, upper_right_bound: C) -> Self
    where
        C: Into<Coordinate>,
    {
        Quadtree {
            inner: Node::Leaf(LeafNode {
                bounds: [lower_left_bound.into(), upper_right_bound.into()],
                value: T::default(),
            }),
        }
    }

    pub fn insert<C>(&mut self, value: T, point: C)
    where
        C: Into<Coordinate>,
    {
        self.inner.insert_value(value, point.into());
    }

    pub fn get<C>(&self, point: C) -> &T
    where
        C: Into<Coordinate>,
    {
        self.inner.read_value(point.into())
    }

    pub fn insert_rect(&mut self, value: T, rect: &Rect) {
        self.inner.insert_value_range(value, rect);
    }
}

#[derive(Debug)]
enum Node<T> {
    Leaf(LeafNode<T>),
    Quad(QuadNode<T>),
}
impl<T> Node<T>
where
    T: Copy + PartialEq,
{
    fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    fn is_quad(&self) -> bool {
        matches!(self, Self::Quad(_))
    }

    fn as_leaf(&self) -> &LeafNode<T> {
        match self {
            Self::Leaf(l) => l,
            _ => panic!(),
        }
    }

    fn as_quad(&self) -> &QuadNode<T> {
        match self {
            Self::Quad(q) => q,
            _ => panic!(),
        }
    }

    fn get_value(&self) -> &T {
        match self {
            Self::Leaf(LeafNode { value, .. }) => value,
            _ => panic!(),
        }
    }

    fn read_value(&self, point: Coordinate) -> &T {
        match self {
            Self::Leaf(LeafNode { value, .. }) => value,
            Self::Quad(QuadNode { nodes, .. }) => nodes
                .into_iter()
                .find(|n| n.contains(point))
                .unwrap()
                .read_value(point),
        }
    }

    fn get_value_mut(&mut self) -> &mut T {
        match self {
            Self::Leaf(LeafNode { value, .. }) => value,
            _ => panic!(),
        }
    }

    fn get_nodes(&self) -> &[Self] {
        match self {
            Self::Quad(QuadNode { nodes, .. }) => nodes,
            _ => panic!(),
        }
    }

    fn get_nodes_mut(&mut self) -> &mut [Self] {
        match self {
            Self::Quad(QuadNode { nodes, .. }) => nodes,
            _ => panic!(),
        }
    }

    fn get_bounds(&self) -> &Rect {
        match self {
            Self::Leaf(LeafNode { bounds, .. }) => bounds,
            Self::Quad(QuadNode { bounds, .. }) => bounds,
        }
    }

    fn contains(&self, point: Coordinate) -> bool {
        let bounds = self.get_bounds();
        bounds[0].0 <= point.0
            && point.0 <= bounds[1].0
            && bounds[0].1 <= point.1
            && point.1 <= bounds[1].1
    }

    fn split(&mut self) {
        if self.is_quad() {
            return;
        }
        let inner = self.as_leaf();
        assert!(
            inner.bounds[1].0 - inner.bounds[0].0 > 0,
            "node is too fine to split"
        );
        assert!(
            inner.bounds[1].1 - inner.bounds[0].1 > 0,
            "node is too fine to split"
        );
        let h_mid = (inner.bounds[1].0 + inner.bounds[0].0) / 2;
        let v_mid = (inner.bounds[1].1 + inner.bounds[0].1) / 2;
        let new_nodes = split_rect(&inner.bounds, h_mid, v_mid)
            .into_iter()
            .map(|b| {
                Self::Leaf(LeafNode {
                    value: inner.value,
                    bounds: b,
                })
            })
            .collect();
        *self = Self::Quad(QuadNode {
            bounds: inner.bounds,
            nodes: new_nodes,
        });
    }

    fn consolidate(&mut self) {
        if self.is_leaf() {
            return;
        }
        let inner = self.as_quad();
        *self = Self::Leaf(LeafNode {
            bounds: inner.bounds,
            value: *inner.nodes[0].get_value(),
        });
    }

    fn insert_value(&mut self, value: T, point: Coordinate) -> bool {
        let bounds = self.get_bounds();
        if point == bounds[0] && point == bounds[1] {
            *self.get_value_mut() = value;
            return true;
        }
        self.split();
        let is_leaf = self
            .get_nodes_mut()
            .into_iter()
            .find(|n| n.contains(point))
            .unwrap()
            .insert_value(value, point);
        if is_leaf
            && self
                .get_nodes()
                .into_iter()
                .all(|n| n.is_leaf() && n.get_value() == &value)
        {
            self.consolidate();
        }
        self.is_leaf()
    }

    fn insert_value_range(&mut self, value: T, bounds: &Rect) -> bool {
        if self.get_bounds() == bounds {
            *self = Self::Leaf(LeafNode {
                bounds: *bounds,
                value,
            });
            return true;
        }
        self.split();
        for node in self.get_nodes_mut() {
            if let Some(intersection) = rect_intersection(node.get_bounds(), &bounds) {
                node.insert_value_range(value, &intersection);
            }
        }
        if self
            .get_nodes()
            .into_iter()
            .all(|n| n.is_leaf() && n.get_value() == &value)
        {
            self.consolidate();
        }
        self.is_leaf()
    }
}

#[derive(Debug)]
struct LeafNode<T> {
    bounds: Rect,
    value: T,
}

#[derive(Debug)]
struct QuadNode<T> {
    bounds: Rect,
    nodes: Vec<Node<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinate(u32, u32);
impl From<(u32, u32)> for Coordinate {
    fn from(value: (u32, u32)) -> Self {
        Self(value.0, value.1)
    }
}

type Rect = [Coordinate; 2];

fn split_rect(bounds: &Rect, h_mid: u32, v_mid: u32) -> Vec<Rect> {
    let corners = [
        bounds[0],
        Coordinate(bounds[1].0, bounds[0].1),
        Coordinate(bounds[0].0, bounds[1].1),
        bounds[1],
    ];
    let mut result = Vec::with_capacity(4);
    if corners[0].0 <= h_mid && corners[0].1 <= v_mid {
        result.push([
            Coordinate(bounds[0].0, bounds[0].1),
            Coordinate(h_mid, v_mid),
        ]);
    }
    if corners[1].0 > h_mid && corners[1].1 <= v_mid {
        result.push([
            Coordinate(h_mid + 1, bounds[0].1),
            Coordinate(bounds[1].0, v_mid),
        ]);
    }
    if corners[2].0 <= h_mid && corners[2].1 > v_mid {
        result.push([
            Coordinate(bounds[0].0, v_mid + 1),
            Coordinate(h_mid, bounds[1].1),
        ]);
    }
    if corners[3].0 > h_mid && corners[3].1 > v_mid {
        result.push([
            Coordinate(h_mid + 1, v_mid + 1),
            Coordinate(bounds[1].0, bounds[1].1),
        ]);
    }
    result
}

fn rect_intersection(left: &Rect, right: &Rect) -> Option<Rect> {
    let x_range = (
        u32::max(left[0].0, right[0].0),
        u32::min(left[1].0, right[1].0),
    );
    if x_range.0 > x_range.1 {
        return None;
    }
    let y_range = (
        u32::max(left[0].1, right[0].1),
        u32::min(left[1].1, right[1].1),
    );
    if y_range.0 > y_range.1 {
        return None;
    }
    Some([
        Coordinate(x_range.0, y_range.0),
        Coordinate(x_range.1, y_range.1),
    ])
}

#[cfg(test)]
mod tests {
    use super::Quadtree;

    #[test]
    fn quadtree_can_be_created() {
        let qtree: Quadtree<bool> = Quadtree::new((0, 0), (8, 8));
        assert!(!*qtree.get((0, 0)));
    }

    mod nodes {
        use super::super::{Coordinate, LeafNode, Node, Rect};

        #[test]
        fn contains_works_correctly() {
            let leaf_node = Node::Leaf(LeafNode {
                bounds: [Coordinate(0, 0), Coordinate(0, 0)],
                value: 0,
            });
            assert!(leaf_node.contains(Coordinate(0, 0)));
            assert!(!leaf_node.contains(Coordinate(1, 0)));
            assert!(!leaf_node.contains(Coordinate(1, 1)));
            assert!(!leaf_node.contains(Coordinate(0, 1)));
        }

        #[test]
        fn you_can_split_leaves() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(4, 4)],
                value: 0,
            });
            node.split();
            assert_eq!(node.get_bounds(), &[Coordinate(1, 1), Coordinate(4, 4)]);
            let child_bounds: Vec<&Rect> = node
                .get_nodes()
                .into_iter()
                .map(|n| n.get_bounds())
                .collect();
            assert_eq!(
                child_bounds,
                &[
                    &[Coordinate(1, 1), Coordinate(2, 2)],
                    &[Coordinate(3, 1), Coordinate(4, 2)],
                    &[Coordinate(1, 3), Coordinate(2, 4)],
                    &[Coordinate(3, 3), Coordinate(4, 4)],
                ]
            );
        }

        #[test]
        fn you_can_split_uneven_leaves() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(5, 5)],
                value: 0,
            });
            node.split();
            let child_bounds: Vec<&Rect> = node
                .get_nodes()
                .into_iter()
                .map(|n| n.get_bounds())
                .collect();
            assert_eq!(
                child_bounds,
                &[
                    &[Coordinate(1, 1), Coordinate(3, 3)],
                    &[Coordinate(4, 1), Coordinate(5, 3)],
                    &[Coordinate(1, 4), Coordinate(3, 5)],
                    &[Coordinate(4, 4), Coordinate(5, 5)],
                ]
            );
        }

        #[test]
        #[should_panic(expected = "node is too fine to split")]
        fn you_cant_split_single_point_leaves() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(0, 0), Coordinate(0, 0)],
                value: 0,
            });
            node.split();
        }

        #[test]
        fn insert_adds_the_value() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(1, 1)],
                value: 0,
            });
            node.insert_value(1, Coordinate(1, 1));
            assert_eq!(node.read_value(Coordinate(1, 1)), &1);
        }

        #[test]
        fn insert_splits_if_it_needs_to() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(2, 2)],
                value: 0,
            });
            node.insert_value(1, Coordinate(1, 1));
            assert_eq!(node.read_value(Coordinate(1, 1)), &1);
            assert_eq!(node.read_value(Coordinate(1, 2)), &0);
            assert_eq!(node.read_value(Coordinate(2, 1)), &0);
            assert_eq!(node.read_value(Coordinate(2, 2)), &0);
        }

        #[test]
        fn insert_consolidates_if_it_can() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(2, 2)],
                value: 0,
            });
            node.insert_value(1, Coordinate(1, 1));
            assert!(node.is_quad());
            node.insert_value(1, Coordinate(1, 2));
            node.insert_value(1, Coordinate(2, 1));
            node.insert_value(1, Coordinate(2, 2));
            assert!(node.is_leaf());
        }

        #[test]
        fn insert_range_adds_the_value() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(1, 1)],
                value: 0,
            });
            node.insert_value_range(1, &[Coordinate(1, 1), Coordinate(1, 1)]);
            assert_eq!(node.read_value(Coordinate(1, 1)), &1);
        }

        #[test]
        fn insert_range_splits_if_it_needs_to() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(2, 2)],
                value: false,
            });
            node.insert_value_range(true, &[Coordinate(1, 1), Coordinate(1, 1)]);
            assert_eq!(node.read_value(Coordinate(1, 1)), &true);
            assert_eq!(node.read_value(Coordinate(1, 2)), &false);
            assert_eq!(node.read_value(Coordinate(2, 1)), &false);
            assert_eq!(node.read_value(Coordinate(2, 2)), &false);
        }

        #[test]
        fn insert_range_consolidates_if_it_can() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(2, 2)],
                value: false,
            });
            node.insert_value(true, Coordinate(1, 1));
            assert!(node.is_quad());
            node.insert_value_range(true, &[Coordinate(1, 1), Coordinate(2, 2)]);
            assert!(node.is_leaf());
        }

        #[test]
        fn insert_range_handles_rectangles() {
            let mut node = Node::Leaf(LeafNode {
                bounds: [Coordinate(1, 1), Coordinate(5, 5)],
                value: (0, 0, 0),
            });
            node.insert_value_range((255, 0, 255), &[Coordinate(1, 1), Coordinate(5, 2)]);
            dbg!(&node);
            // TODO: Figure out how to better assert tree
        }
    }
}
