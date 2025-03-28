#![allow(dead_code)]
use crate::error::NodeError;

#[cfg(not(feature = "std"))]
use core::{
    cell::RefCell,
    fmt,
    fmt::{Debug, Display},
    iter::Iterator,
    mem,
};

#[cfg(not(feature = "std"))]
use rclite::Rc;
use tracing::instrument;

#[cfg(not(feature = "std"))]
use alloc::{self, vec::Vec};

#[cfg(feature = "std")]
use std::{
    cell::RefCell,
    fmt,
    fmt::{Debug, Display},
    iter::Iterator,
    mem,
    rc::{Rc, Weak},
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
/// let node: NodeRef<i32> = Node::leaf(42, None);
/// ```
pub type NodeRef<T> = Rc<RefCell<Node<T>>>;

#[cfg(not(feature = "std"))]
pub type ParentRc<T> = Rc<T>;

#[cfg(feature = "std")]
pub type ParentRc<T> = Weak<T>;

/// A reference-counted specifically to it's parent
///
///
///  ### `no_std`
/// [`Rc<RefCell<Node<T>>>`] is used because:
/// - [`Rc<T>`] enables multiple owners.
/// - [`RefCell<T>`] allows for interior mutability.
///
///  ### `std`
/// [`Weak<RefCell<Node<T>>>`] is used because:
/// - [`Weak<T>`] non-owning reference.
/// - [`RefCell<T>`] allows for interior mutability, in case we upgrade [`Weak`] to [`Rc`].
///
///
/// # Example:
/// ```
/// let node: NodeRef<i32> = Node::leaf(42, None);
/// ```
pub type PrevNodeRef<T> = ParentRc<RefCell<Node<T>>>;

/// Node is a tree data structure element which can either be a `Leaf` (holding just a value, and it's parent reference)
/// or `Parent` (holding reference to children, parent, and value)
///
/// ## Example
///
/// ### Creating a leaf Node
/// ```
/// let node = Node::leaf(true, None);
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
/// let node = Node::Parent { value : true,
///                           prev : None,
///                           next : vec![] };
/// let _ = Node::insert(&node, false)?;
///
/// ```
///
/// ## Layout
/// ```text    
///   (1 bytes)   (3 bytes)   (n bytes)            (8 bytes)                            (24 bytes)
/// ┌───────────┬───────────┬───────────┬────────────────────────────────────┬────────────────────────────────────────┐
/// │  Discrimt │  Padding  │     T     │      Option<ParentNodeRef<T>>      │             Vec<NodeRef<T>>            │
/// └─────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
/// ```
///
///
/// ## `no_std` Layout
/// ```text    
///   (1 bytes)   (3 bytes)   (n bytes)            (8 bytes)                            (24 bytes)
/// ┌───────────┬───────────┬───────────┬──────────────────────────────┬────────────────────────────────────────┐
/// │  Discrimt │  Padding  │     T     │      Option<NodeRef<T>>      │             Vec<NodeRef<T>>            │
/// └───────────────────────────────────────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// - `Discriminant (1 byte)`: Stores enum variant (`Leaf` = `0`, `Parent` = `1`).
/// - `Padding (3 bytes)`: Ensures memory alignment.
/// - `Value (N bytes)`: Stores the data of type `T` (e.g., 4 bits are allocated when T is `i32`).
/// - `prev (8 bytes)`: `Option<Rc<RefCell<Node<T>>>>`, storing a pointer.
/// - `next (24 bytes)`: `Vec<Rc<RefCell<Node<T>>>>`, storing a `Vec` (pointer, length, capacity).
///
#[derive(Clone)]
#[repr(u8)]
pub enum Node<T> {
    Leaf {
        prev: Option<PrevNodeRef<T>>,
        value: T,
    },
    Parent {
        value: T,
        prev: Option<PrevNodeRef<T>>,
        next: Vec<NodeRef<T>>,
    },
}

impl<T> Debug for Node<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf { value, .. } => f.debug_struct("Leaf").field("value", value).finish(),
            Self::Parent { value, prev, next } => f
                .debug_struct("Parent")
                .field("value", value)
                .field("prev", &prev.as_ref().map(|p| format!("{:p}", p)))
                .field("next_count", &next.len()) // Show number of children instead of pointer
                .finish(),
        }
    }
}

impl<T> Display for Node<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf { value, .. } => write!(f, "Leaf({:?})", value),
            Self::Parent { value, prev, next } => write!(
                f,
                "Parent(value = {:?}, prev = {}, children = {})",
                value,
                if prev.is_some() { "Some" } else { "None" },
                next.len()
            ),
        }
    }
}

impl<T> Node<T>
where
    T: Debug,
{
    /// Create [`Node::Parent`] instance
    #[inline]
    #[instrument(level = "trace")]
    pub fn parent(value: T) -> NodeRef<T> {
        Rc::new(RefCell::new(Node::Parent {
            value,
            prev: None,
            next: vec![],
        }))
    }

    /// Create [`Node::Leaf`] instance
    #[inline]
    #[instrument(level = "trace")]
    pub fn leaf(value: T, prev: Option<NodeRef<T>>) -> NodeRef<T> {
        Rc::new(RefCell::new(Node::Leaf {
            value,
            #[cfg(feature = "std")]
            prev: prev.map(|p| Rc::downgrade(&p)),
            #[cfg(not(feature = "std"))]
            prev,
        }))
    }

    /// Converts a [`Node::Leaf`] node into a [`Node::Parent`] node with an initial child.
    ///
    /// # Parameters:
    /// - `parent`: A reference-counted node that will be converted.
    /// - `node`: A reference-counted node that
    ///
    /// # Return:
    /// - Result where on success empty tuple, and [`NodeError::ParentUpgradeNotAllowed`] if the node is already a [`Node::Parent`].
    ///
    /// # Example:
    /// ```
    /// let mut leaf = Node::leaf(42, None);
    ///
    /// // insert child node into leaf to make it parent.
    /// let child = Node::leaf(100, Some(leaf.clone()));
    /// Node::upgrade(&leaf, &child)?;
    /// ```
    #[inline]
    #[instrument(level = "trace")]
    pub fn upgrade(parent: &NodeRef<T>, node: &NodeRef<T>) -> Result<(), NodeError>
    where
        T: Debug + Default + Clone,
    {
        parent.borrow_mut().upgrade_inner(node)
    }

    fn upgrade_inner(&mut self, node: &NodeRef<T>) -> Result<(), NodeError>
    where
        T: Default + Clone,
    {
        match self {
            Self::Leaf { value, prev } => {
                // we have a mutable refrence of T, to which we need to
                let leaf_value = mem::take(value);
                let prev = mem::take(prev);
                *self = Self::Parent {
                    value: leaf_value,
                    prev,
                    next: vec![Rc::clone(node)],
                };
                Ok(())
            }
            _ => Err(NodeError::ParentUpgradeNotAllowed), // Assuming you have this error variant
        }
    }

    /// Converts a [`Node::Parent`] node into a [`Node::Leaf`] node, discarding its children.
    ///
    /// ### Parameters
    /// `node`: refrence count to the orginal node that will be downgraded to [`Node::Leaf`]
    ///
    /// ### Return
    /// Result of an empty tuple on success and
    /// [`NodeError::DowngradeNotParent`] if the node is already a [`Node::Leaf`],
    /// and [`NodeError::IllegalDowngradeWithChildren`] if node is still has children.
    ///
    /// ### Example:
    /// ```
    /// let root = Node::parent(42);
    /// let child = Node::insert(&root, 69);
    /// // upgrades child from leaf -> parent
    /// let gc = Node::insert(&child, 420);
    /// // By poping childs only child node will
    /// // auto switched to Leaf
    /// Node::pop(&child, &gc)?;
    /// ```
    #[inline]
    #[instrument(level = "trace")]
    pub fn downgrade(node: &NodeRef<T>) -> Result<(), NodeError>
    where
        T: Default + Debug,
    {
        node.borrow_mut().downgrade_inner()
    }

    fn downgrade_inner(&mut self) -> Result<(), NodeError>
    where
        T: Default,
    {
        match self {
            Self::Parent { value, prev, next } => {
                //
                let children = next.len();
                if children != 0 {
                    return Err(NodeError::IllegalDowngradeWithChildren(children));
                }

                if let Some(parent) = prev.take() {
                    let parent_value = mem::take(value);
                    *self = Self::Leaf {
                        prev: Some(parent),
                        value: parent_value,
                    };
                    Ok(())
                } else {
                    Err(NodeError::RootDowngradeNotAllowed)
                }
            }
            _ => Err(NodeError::DowngradeNotParent), // Assuming you have this error variant
        }
    }
}

impl<T> Node<T> {
    /// Check if node is specific [`Node::Parent`] instance that classify a root node.
    /// ### Classify A Root
    /// - prev is None
    /// - node is a Parent
    /// - zero to many children
    ///
    /// ### Returns
    /// - `bool` if node is not a root.
    #[inline]
    pub fn is_root(&self) -> bool {
        match self {
            Self::Parent { prev, .. } => prev.is_none(),
            _ => false,
        }
    }

    /// Expects the node to be a root, or returns an error.
    ///
    /// ### Return
    /// - Result of an empty tuple or [`NodeError::ExpectedARootNode`] if the node is not a root.
    #[inline]
    pub fn expect_root(&self) -> Result<(), NodeError> {
        if self.is_root() {
            Ok(())
        } else {
            Err(NodeError::ExpectedARootNode)
        }
    }

    /// Check if node is ``Leaf`` instance
    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf { .. })
    }

    /// Expects the node to be a leaf, or returns an error.
    ///
    /// ### Return
    /// - Result of an empty tuple or [`NodeError::ExpectedALeafNode`] if the node is not a leaf.
    #[inline]
    pub fn expect_leaf(&self) -> Result<(), NodeError> {
        if self.is_leaf() {
            Ok(())
        } else {
            Err(NodeError::ExpectedALeafNode)
        }
    }

    /// ### Return
    /// - `bool` that checks if Node instance has children
    #[inline]
    pub fn has_children(&self) -> bool {
        match self {
            Self::Parent { next, .. } => !next.is_empty(),
            Self::Leaf { .. } => false,
        }
    }

    /// ### Return
    /// - `&T` of the [`Node<T>`]
    #[inline]
    pub fn value(&self) -> &T {
        match self {
            Self::Parent { value, .. } => value,
            Self::Leaf { value, .. } => value,
        }
    }

    /// ### Return
    /// - list of [`NodeRef<T>`]
    #[inline]
    pub fn children(&self) -> &[NodeRef<T>] {
        match self {
            Self::Parent { next, .. } => next,
            _ => &[],
        }
    }

    /// ### Return
    /// - Excpets a return list of [`NodeRef<T>`] else return [`NodeError::ExpectedChildren`]
    #[inline]
    pub fn expect_children(&self) -> Result<&[NodeRef<T>], NodeError> {
        match self {
            Self::Parent { next, .. } => {
                if next.is_empty() {
                    Err(NodeError::ExpectedChildren)
                } else {
                    Ok(next)
                }
            }
            _ => Err(NodeError::ExpectedChildren),
        }
    }

    /// ### Returns
    /// - Cloned refrence of the parent node.
    #[inline]
    pub fn prev(&self) -> Option<PrevNodeRef<T>> {
        match self {
            Self::Parent { prev, .. } => prev.clone(),
            Self::Leaf { prev, .. } => prev.clone(),
        }
    }

    /// ### Return
    /// Assert a cloned refrence of the parent node, or else return [`NodeError::ParentNodeNotFound`].
    #[inline]
    pub fn expect_prev(&self) -> Result<PrevNodeRef<T>, NodeError> {
        match self {
            Self::Parent { prev, .. } => prev.clone().ok_or(NodeError::ParentNodeNotFound),
            Self::Leaf { prev, .. } => prev.clone().ok_or(NodeError::ParentNodeNotFound),
        }
    }
}

impl<T> Node<T>
where
    T: Debug + Default + Clone,
{
    /// Insert [`Node`] with value T within the [`Node`]
    ///
    /// ### Parameters
    /// - `parent`: A refrence to the Node to which will add child to.
    /// - `value`: A generic value type.
    ///
    /// ### Return
    /// - Result of a [`NodeRef<T>`] to the newly inserted child node, or [`NodeError::ParentUpgradeNotAllowed`] if the node is already a `Parent`.
    ///
    /// ### Example
    /// ```
    /// let root = Node::parent(1);
    /// let child1 = Node::insert(root, 2)?;
    /// let _ = Node::insert(child1, 3)?;
    /// let _ = Node::insert(child1, 4)?;
    /// ```
    #[instrument(level = "info")]
    pub fn insert(parent: &NodeRef<T>, value: T) -> Result<NodeRef<T>, NodeError> {
        let node = Node::leaf(value, Some(Rc::clone(parent)));
        Node::inner_insert(parent, &node)?;
        Ok(node)
    }

    /// Insert [`NodeRef`] within the prarent [`NodeRef`]`
    ///
    /// ### Parameters
    /// - `parent`: A refrence to the Node to which will add child to.
    /// - `node`: A referecne to child node.
    ///
    /// ### Return
    /// - Result of a empty tuple, or [`NodeError`]
    ///
    /// ### Example
    /// ```
    /// let root = Node::parent(1);
    /// // allocat memeory for child with no parent
    /// let child = Node::leaf(2, None);
    ///
    /// // insert the child into the parent root node
    /// let _ = Node::insert_node(&root, &child)?;
    /// ```
    #[instrument(level = "info")]
    #[cfg(feature = "std")]
    pub fn insert_node(parent: &NodeRef<T>, node: &NodeRef<T>) -> Result<(), NodeError> {
        let mut n = node.borrow_mut();
        match &mut *n {
            Node::Leaf { prev, .. } | Node::Parent { prev, .. } => {
                *prev = Some(NodeRef::downgrade(parent));
            }
        }
        Node::inner_insert(parent, node)?;
        Ok(())
    }

    /// Insert [`NodeRef`] within the prarent [`NodeRef`]`
    ///
    /// ### Parameters
    /// - `parent`: A refrence to the Node to which will add child to.
    /// - `node`: A referecne to child node.
    ///
    /// ### Return
    /// - Result of a empty tuple, or [`NodeError`]
    ///
    /// ### Example
    /// ```
    /// let root = Node::parent(1);
    /// // allocat memeory for child with no parent
    /// let child = Node::leaf(2, None);
    ///
    /// // insert the child into the parent root node
    /// let _ = Node::insert_node(&root, &child)?;
    /// ```
    #[instrument(level = "info")]
    #[cfg(not(feature = "std"))]
    pub fn insert_node(parent: &NodeRef<T>, node: &NodeRef<T>) -> Result<(), NodeError> {
        let mut n = node.borrow_mut();
        match &mut *n {
            Node::Leaf { prev, .. } | Node::Parent { prev, .. } => {
                *prev = Some(NodeRef::clone(parent));
            }
        }
        Node::inner_insert(parent, node)?;
        Ok(())
    }

    fn inner_insert(parent: &NodeRef<T>, node: &NodeRef<T>) -> Result<(), NodeError> {
        let mut p = parent.borrow_mut();
        // Get mutable access to the parent
        match &mut *p {
            Node::Leaf { .. } => {
                drop(p);
                // If parent is a leaf, upgrade it to a parent and add this node as a child
                Node::upgrade(parent, node)?;
            }
            Node::Parent { next, .. } => {
                // If parent is already a parent, just add this node to its children
                next.push(Rc::clone(node));
            }
        }

        // Return the new child node
        Ok(())
    }

    /// Removes a child node from its parent [`Node::Parent`].
    ///
    /// ### Parameters
    /// - `parent`: A refrence to the Node to which will add child to.
    /// - `child`: A reference to the child node to be removed.
    ///
    /// ### Returns
    /// - A result of a [`bool`]: where `true` If the child was successfully removed.
    ///   and `false` If the child was not found among this node's children.
    ///
    /// If removing the child results in an empty parent, the parent **downgrades** into a [`Node::Leaf`].
    ///
    /// # Example
    ///
    /// ```
    /// let root = Node::parent(1);
    /// let child = Node::insert(&root, 2)?;
    /// let grand_child = Node::insert(&child, 3);
    /// let result = Node::pop(&child, &grand_child)?;
    /// assert!(result); // Successfully removed
    /// ```
    #[instrument(level = "trace")]
    pub fn pop(parent: &NodeRef<T>, child: &NodeRef<T>) -> Result<bool, NodeError>
    where
        T: Default + Clone + Copy,
    {
        parent.borrow_mut().inner_pop(child)
    }

    fn inner_pop(&mut self, child: &NodeRef<T>) -> Result<bool, NodeError>
    where
        T: Default + Clone + Copy,
    {
        match self {
            Self::Leaf { .. } => Err(NodeError::NotAParent),
            Self::Parent { next, prev, .. } => {
                // Find the position of the child in the vector
                let position = next.iter().position(|c| Rc::ptr_eq(c, child));

                if let Some(index) = position {
                    // Remove the child at the found position
                    next.remove(index);

                    {
                        let mut c = child.borrow_mut();
                        match &mut *c {
                            Self::Parent { prev, .. } | Self::Leaf { prev, .. } => {
                                prev.take();
                            }
                        }
                    }

                    // Update the child's parent reference (set to None)
                    if next.is_empty() && prev.is_some() {
                        Node::downgrade_inner(self)?;
                    }

                    Ok(true)
                } else {
                    // Child not found in this parent's children
                    Ok(false)
                }
            }
        }
    }
}

impl<T> From<Node<T>> for NodeRef<T> {
    fn from(node: Node<T>) -> Self {
        Rc::new(RefCell::new(node))
    }
}

impl<T> Node<T> {
    pub fn iter(node: NodeRef<T>) -> NodeIter<T> {
        NodeIter::new(node)
    }
}

pub struct NodeIter<T> {
    queue: Vec<NodeRef<T>>,
}

impl<T> NodeIter<T> {
    pub fn new(node: NodeRef<T>) -> NodeIter<T> {
        let queue = vec![node];
        NodeIter { queue }
    }
}
impl<T> Iterator for NodeIter<T> {
    type Item = NodeRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.queue.first().cloned() {
            self.queue.remove(0); // Remove first element (FIFO)

            if let Node::Parent { next, .. } = &*item.borrow() {
                self.queue.extend(next.clone()); // Add children to queue
            }

            Some(item)
        } else {
            None
        }
    }
}
