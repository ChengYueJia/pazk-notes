// Proof is a tree, only contain the hash values from target leaf to root with related brather-nodes.
// Meanwhile, half of the tree can be calculated by the known leaf value..
// So according the Figure 7.1(from zkbook), it's quite easy to find that just need to return the hasher from
// brather-nodes(each layer has only one!), the left infos will be calculated by verifier.
// And totally needs h hash values.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Proof {
    pub children: Vec<u64>, // the children from left to root. aka evals
    pub root: u64,          // root hash. aka cm
}
