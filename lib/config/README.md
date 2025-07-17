# Why this part of the code is a little complicated

The main problem we are trying to solve in a decentralised network is knowing who to trust.
Signatures can prove that a node sent a given post, but who do we listen for posts from, or given them to?

We can start with the base case:

    1. We trust ourselves
    2. We trust the bootstrap node

Without this base case, no further conclusions can be made. When we wish to send a post to the network, we send it to the bootstrap node because we trust it. The bootstrap node, similarly, has nodes that it trusts, so the chain continues.

This process works in reverse where we receive posts from the bootstraps' secondary peers.

When sharing posts, we make sure to sign a signature to prove that we sent it to a given peer and add it to the post's history. Later, when an untrusted node wants to connect to us, it can give us proof that it received a post from one of our peers (because they did the same thing). Now, given that we trust our peer, and that they can prove that our peer trusts them, by commutative properties, we can also trust this new node.

By further scoring our peers, we can trust the best n nodes to save on bandwidth.

This comes with the drawback that the bootstrap node will likely be surrounded by undesired nodes that are scored lowly by their peers. Unfortunately, this is an issue that can't really be solved easily. New nodes will have to spend time traversing the network if they want to build their reputation, as designed. 

Rules:

1. We only send & receive to trusted nodes
2. We trust nodes that share a common intermediary node (that is scored high enough)
3. We only keep the top 16 trusted nodes with the highest scores
4. If we receive a trust request that scores higher than the intermediary node but our trusted list is full, then we drop the intermediary node in preference for the alternative node.
5. We score nodes based on their posts, regardless if they are trusted or not.
6. We only send a trust request if a node is 'statistically significant' -> elo scores.
7. When sending a post to an intermediary node, we make sure to limit the post history to who we received it from and us.
8. If we received a post that we have already seen & sent out, then we ignore it.


