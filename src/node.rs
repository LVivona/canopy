#![allow(dead_code)]
use crate::error::NodeError;
use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

/// A reference-counted, mutable reference to a `Node<T>`.
/// This allows multiple parts of the program to hold the node, and
/// share ownership of the original, while ensuring interior mutability.
///
///
/// [`Rc<RefCell<Node<T>>>`] is used because:
/// - [`Rc<T>`] enables multiple owners.
/// - [`RefCell<T>`] allows for interior mutability.
///
/// # Example:
/// ```
/// let node: NodeRef<i32> = Rc::new(RefCell::new(Node::leaf(42)));
/// ```

pub type NodeRef<T> = Rc<RefCell<Node<T>>>;

/// Node is a tree data structure element which can either be a `Leaf` (holding just a value)
/// or `Parent` (holding reference to children)
///
/// ## Example
///
/// ### Creating a leaf Node
/// ```
/// let node = Node::leaf(true);
/// assert!(leaf.is_leaf());
/// ```
///
/// ### Creating a Root Node
/// ```
/// let node = Node::parent(true);
/// assert!(leaf.is_root());
/// ```
/// ### Link nodes together
/// ```
/// let child = Node::leaf(true);
/// let node = Node::Parent { value : true,
///                           prev : None,
///                           next : vec![Rc::new(RefCell::new(child))] };
///
/// ```
///
/// ## Layout
/// ```text    
///   (1 bytes)   (3 bytes)   (n bytes)            (8 bytes)                            (24 bytes)
/// ┌───────────┬───────────┬───────────┬──────────────────────────────┬────────────────────────────────────────┐
/// │  Discrimt │  Padding  │     T     │      Option<NodeRef<T>>      │        Option<Vec<NodeRef<T>>>         │
/// └───────────────────────────────────────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// - `Discriminant (1 byte)`: Stores enum variant (`Leaf` = `0`, `Parent` = `1`).
/// - `Padding (3 bytes)`: Ensures memory alignment.
/// - `Value (N bytes)`: Stores the data of type `T` (e.g., 4 bits are allocated when T is `i32`).
/// - `prev (8 bytes)`: `Option<Rc<RefCell<Node<T>>>>`, storing a pointer.
/// - `next (24 bytes)`: `Option<Vec<Rc<RefCell<Node<T>>>>>`, storing a `Vec` (pointer, length, capacity).
///
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum Node<T> {
    Leaf {
        value: T,
    },
    Parent {
        value: T,
        prev: Option<NodeRef<T>>,
        next: Option<Vec<NodeRef<T>>>,
    },
}

impl<T> Display for Node<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Leaf { value } => write!(f, "{:?}", value),
            Self::Parent { value, prev, next } => write!(
                f,
                " Parent{{ value = {:?} prev = {:?}, next = {:?} }}",
                value, prev, next
            ),
        }
    }
}

impl<T> std::default::Default for Node<T>
where
    T: Default,
{
    fn default() -> Self {
        Node::Parent {
            value: T::default(),
            prev: None,
            next: None,
        }
    }
}

impl<T> Node<T> {
    #[inline]
    /// Create [`Node::Parent`] instance
    pub fn parent(value: T) -> Node<T> {
        Node::Parent {
            value,
            prev: None,
            next: None,
        }
    }

    #[inline]
    /// Create [`Node::Leaf`] instance
    pub fn leaf(value: T) -> Node<T> {
        Node::Leaf { value }
    }

    #[inline]
    /// Converts a `Leaf` node into a `Parent` node with an initial child.
    ///
    /// # Parameters:
    /// - `node`: A reference-counted node that will be the first child.
    ///
    /// # Errors:
    /// - Returns [`NodeError::UpgradeNotLeaf`] if the node is already a `Parent`.
    ///
    /// # Example:
    /// ```
    /// let mut leaf = Node::leaf(42);
    /// let child = Rc::new(RefCell::new(Node::leaf(100)));
    ///
    /// leaf.upgrade(child).unwrap(); // Now it's a `Parent`
    /// ```
    fn upgrade(&mut self, node: NodeRef<T>) -> Result<(), NodeError>
    where
        T: Default + Clone + Copy,
    {
        match self {
            Self::Leaf { value } => {
                // we have a mutable refrence of T, to which we need to
                let leaf_value = std::mem::take(value);
                *self = Self::Parent {
                    value: leaf_value,
                    prev: None,
                    next: Some(vec![node]),
                };
                Ok(())
            }
            _ => Err(NodeError::UpgradeNotLeaf), // Assuming you have this error variant
        }
    }

    #[inline]
    /// Converts a `Parent` node into a `Leaf` node, discarding its children.
    ///
    /// # Errors:
    /// - Returns [`NodeError::DowngradeNotParent`] if the node is already a `Leaf`.
    ///
    /// # Example:
    /// ```
    /// let mut parent = Node::parent(42);
    /// parent.downgrade().unwrap(); // Now it's a `Leaf`
    /// ```
    fn downgrade(&mut self) -> Result<(), NodeError>
    where
        T: Default + Clone + Copy,
    {
        match self {
            Self::Parent { value, .. } => {
                let parent_value = std::mem::take(value);
                *self = Self::Leaf {
                    value: parent_value,
                };
                Ok(())
            }
            _ => Err(NodeError::DowngradeNotParent), // Assuming you have this error variant
        }
    }
}

impl<T> Node<T> {
    #[inline]
    /// Check if node is ``Parent`` instance
    /// ### Classify A Root
    /// - prev is None
    /// - node is a Parent
    /// - zero to many children
    ///
    /// Returns `bool` if node is not a root.
    pub fn is_root(&self) -> bool {
        match self {
            Self::Parent { prev, .. } => prev.is_none(),
            _ => false,
        }
    }

    /// Expects the node to be a root, or returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`NodeError::NotARoot`] if the node is not a root.
    pub fn expect_root(&self) -> Result<(), NodeError> {
        if self.is_leaf() {
            Ok(())
        } else {
            Err(NodeError::NotARoot)
        }
    }

    #[inline]
    /// Check if node is ``Leaf`` instance
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf { .. })
    }

    /// Expects the node to be a leaf, or returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`NodeError::NotALeaf`] if the node is not a leaf.
    pub fn expect_leaf(&self) -> Result<(), NodeError> {
        if self.is_leaf() {
            Ok(())
        } else {
            Err(NodeError::NotALeaf)
        }
    }

    /// Return `bool` that checks if Node instance has children
    #[inline]
    pub fn has_children(&self) -> bool {
        match self {
            Self::Parent { next, .. } => match next {
                Some(v) => !v.is_empty(),
                _ => false,
            },
            Self::Leaf { .. } => false,
        }
    }

    /// Returns `&T` of the [`Node<T>`]
    #[inline]
    pub fn value(&self) -> &T {
        match self {
            Self::Parent { value, .. } => value,
            Self::Leaf { value } => value,
        }
    }

    #[inline]
    fn children(&self) -> &[NodeRef<T>] {
        match self {
            Self::Parent { next, .. } => next.as_deref().unwrap_or(&[]),
            _ => &[],
        }
    }
}

impl<T> Node<T> {
    /// Insert [`Node`] with value T within the [`Node`]
    ///
    /// # Returns:
    /// - A reference to the newly inserted child node.
    /// - [`NodeError::UpgradeNotLeaf`] if the node is already a `Parent`.
    ///
    /// ## Example
    /// ```
    /// let root = Node::parent(1);
    /// let child1 = root.insert(2);
    /// let _ = root.insert(3);
    /// // barrow a mutable refrence to the child node
    /// // and insert ``4`` as child of that node
    /// let _ = grand_child = child1.borrow_mut().insert(4);
    ///
    /// ```
    fn insert(&mut self, value: T) -> Result<NodeRef<T>, NodeError>
    where
        T: Default + Clone + Copy,
    {
        let node = Rc::new(RefCell::new(Node::leaf(value)));
        match self {
            Node::Leaf { .. } => {
                self.upgrade(node.clone())?;
            }
            Node::Parent { next, .. } => {
                if next.is_none() {
                    *next = Some(vec![]);
                }

                if let Some(children) = next {
                    children.push(node.clone());
                }
            }
        }

        Ok(node)
    }

    /// Removes a child node from its parent (`Node::Parent`).
    ///
    /// # Parameters
    /// - `child`: A reference to the child node to be removed.
    ///
    /// # Returns
    ///
    /// - `Ok(true)`: If the child was successfully removed.
    /// - `Ok(false)`: If the child was not found among this node's children.
    /// - [`NodeError::NotAParent`]: If the node is a `Leaf` and cannot have children.
    ///
    /// If removing the child results in an empty parent, the parent **downgrades** into a `Leaf`.
    ///
    /// # Example
    ///
    /// ```
    /// let mut root = Node::parent(1);
    /// let child = root.insert(2).unwrap();
    /// let _ = root.insert(3);
    ///
    /// assert!(root.pop(&child).unwrap()); // Successfully removed
    /// assert!(!root.pop(&child).unwrap()); // Already removed, returns false
    /// ```
    pub fn pop(&mut self, child: &NodeRef<T>) -> Result<bool, NodeError>
    where
        T: Default + Clone + Copy,
    {
        match self {
            Self::Leaf { .. } => Err(NodeError::NotAParent),
            Self::Parent { next, .. } => {
                if let Some(children) = next {
                    // Find the position of the child in the vector
                    let position = children.iter().position(|c| Rc::ptr_eq(c, child));

                    if let Some(index) = position {
                        // Remove the child at the found position
                        children.remove(index);

                        // Update the child's parent reference (set to None)
                        if children.is_empty() {
                            self.downgrade()?;
                        }

                        Ok(true)
                    } else {
                        // Child not found in this parent's children
                        Ok(false)
                    }
                } else {
                    // No children to remove
                    Ok(false)
                }
            }
        }
    }

    /// Detaches this node from its parent.
    ///
    /// If this node has a parent, it is removed from its parent's child list.
    ///
    /// # Returns
    ///
    /// - `Ok(true)`: If the node was successfully detached from its parent.
    /// - `Ok(false)`: If the node has no parent.
    /// - [`NodeError::NotAParent`]: If the node is a `Leaf` and has no children.
    /// - [`NodeError::ParentBorrowed`]: If the parent is already mutably borrowed elsewhere.
    ///
    /// # Example
    ///
    /// ```
    /// let mut root = Node::parent(1);
    /// let child = root.insert(2).unwrap();
    ///
    /// assert!(child.borrow_mut().detach().unwrap()); // Successfully detached
    /// assert!(!child.borrow_mut().detach().unwrap()); // Already detached, returns false
    /// ```
    fn detach(&mut self) -> Result<bool, NodeError>
    where
        T: Default + Clone + Copy,
    {
        match self {
            Self::Leaf { .. } => Err(NodeError::NotAParent),
            Self::Parent { prev, .. } => {
                if let Some(parent_ref) = prev.take() {
                    // Try to borrow the parent mutably
                    if let Ok(mut parent) = parent_ref.try_borrow_mut() {
                        // Find and remove this node from parent's children
                        let self_rc = Rc::new(RefCell::new(self.clone()));
                        parent.pop(&self_rc)?;
                        Ok(true)
                    } else {
                        // Couldn't borrow parent mutably, restore the prev reference
                        *prev = Some(parent_ref);
                        Err(NodeError::ParentBorrowed)
                    }
                } else {
                    // No parent to detach from
                    Ok(false)
                }
            }
        }
    }
    // fn pop(&mut self, value : NodeRef<T>) -> Result<NodeRef<T>, NodeError>  {
    // }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::error::NodeError;

    use super::Node;

    #[test]
    fn node_parent_creation_root() {
        let node = Node::parent(true);
        assert!(node.is_root())
    }

    #[test]
    fn node_parent_creation_leaf() {
        let node = Node::leaf(true);
        assert!(node.is_leaf())
    }

    #[test]
    fn node_root_chidren_has_no_children() {
        let root = Node::parent(true);
        assert!(!root.has_children())
    }

    #[test]
    fn node_root_chidren_has_children() {
        let child = Node::leaf(false);
        let root = Node::Parent {
            value: true,
            prev: None,
            next: Some(vec![Rc::new(RefCell::new(child))]),
        };
        assert!(root.has_children())
    }

    #[test]
    fn exception_non_root_check() {
        let root = Node::parent(true);
        assert!(root.expect_root().is_err())
    }

    #[test]
    fn exception_leaf_check() {
        let root = Node::leaf(true);
        assert!(root.expect_leaf().is_ok())
    }

    #[test]
    fn test_insertion() -> Result<(), NodeError> {
        let mut root = Node::parent(1);
        let _ = root.insert(2)?;
        let child = root.insert(3)?;
        let _ = child.borrow_mut().insert(4)?;
        println!("{:?}", root);
        Ok(())
    }

    #[test]
    fn test_pop() -> Result<(), NodeError> {
        let mut root = Node::parent(1);
        let child = root.insert(2).unwrap();
        let _ = root.insert(3);

        assert!(root.pop(&child).unwrap()); // Successfully removed
        Ok(())
    }
}
