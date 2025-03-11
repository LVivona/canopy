/// Represents the different errors that can occur while working with a `Node`.
/// These errors are used to indicate issues with node manipulation, such as upgrading or downgrading a node,
/// or when a node does not meet the expected type (e.g., leaf, parent, root).
#[derive(Debug)]
#[non_exhaustive]
pub enum NodeError {
    /// Raised when an attempt to upgrade a node that is not a leaf is made.
    UpgradeNotLeaf,

    /// Raised when an attempt to downgrade a node that is not a parent is made.
    DowngradeNotParent,

    /// Raised when a leaf node is expected but a different type of node is encountered.
    NotALeaf,

    /// Raised when a root node is expected but a different type of node is encountered.
    NotARoot,

    /// Raised when a parent node is expected but a different type of node is encountered.
    NotAParent,

    /// Raised when trying to mutate a parent node that is already borrowed mutably elsewhere.
    ParentBorrowed,
}


impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpgradeNotLeaf => write!(f, "Upgrage failure Node was not not a leaf"),
            Self::DowngradeNotParent => write!(f, "Downgrade failure Node was not not a parent"),
            Self::NotALeaf => write!(f, "Expected a leaf node"),
            Self::NotARoot => write!(f, "Expected a root node"),
            Self::NotAParent => write!(f, "Expected a node Node::Parent"),
            Self::ParentBorrowed => write!(f, "Parent node is already borrowed mutably"),
        }
    }
}

impl std::error::Error for NodeError {}