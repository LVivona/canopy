/// Represents the different errors that can occur while working with a `Node`.
/// These errors are used to indicate issues with node manipulation, such as upgrading or downgrading a node,
/// or when a node does not meet the expected type (e.g., leaf, parent, root).
#[cfg(not(feature = "std"))]
use core::{fmt, error::Error};

#[cfg(feature = "std")]
use std::{fmt, error::Error};

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeError {

    /// Raised when an attempt to downgrade a node that is not a parent is made.
    DowngradeNotParent,

    /// Raised when attempting to downgrade a root node, which is not allowed.
    RootDowngradeNotAllowed,

    /// Raised when parent still has children within it's refrence
    IllegalDowngradeWithChildren(usize),

    /// Raise when attempting to upgrade node, which is a not allowed.
    ParentUpgradeNotAllowed,

    /// Raised when a leaf node is expected but a different type of node is encountered.
    ExpectedALeafNode,

    /// Raised when a root node is expected but a different type of node is encountered.
    ExpectedARootNode,

    /// Raised when a parent node is expected but a different type of node is encountered.
    NotAParent,

    /// Raised when trying to mutate a parent node that is already borrowed mutably elsewhere.
    AlreadyBorrowed,

    /// Raised when they expected the a parent to be there
    ParentNodeNotFound,

    /// Raised when children are expected
    ExpectedChildren
}


impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParentUpgradeNotAllowed => write!(f, "Upgrage failure Node was not not a leaf"),
            Self::DowngradeNotParent => write!(f, "Downgrade failure Node was not not a parent"),
            Self::RootDowngradeNotAllowed => write!(f, "Root node cannot be downgraded"),
            Self::IllegalDowngradeWithChildren(children) => write!(f, "Downgrade failure, node still has reference to {} children", children),
            Self::ExpectedALeafNode => write!(f, "Expected a leaf node"),
            Self::ExpectedARootNode => write!(f, "Expected a root node"),
            Self::NotAParent => write!(f, "Expected a node Node::Parent"),
            Self::AlreadyBorrowed => write!(f, "Node is already borrowed mutably"),
            Self::ParentNodeNotFound => write!(f, "Parent not found"),
            Self::ExpectedChildren => write!(f, "Expected the node to have children")
        }
    }
}

impl Error for NodeError {}