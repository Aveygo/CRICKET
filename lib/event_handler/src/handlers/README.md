
# Protocol (ignoring signatures)

Every node has a public key, which is used to identify themselves
Node {
    public_key: String
}

A node can request another node for their posts 
PostRequest {}

This is how the posts are returned
PostResponse {
    posts: Vec<
        Post {
            author: Node,
            content: String,
            signature: String
        }
    >
}

Eventually, a node will request for secondary peers
SecondaryPeerRequest {}

And if the node is trusted, will get the public keys of their peers
SecondaryPeerResponse {
    nodes: Vec<
        Node
    >
}

# Adding an event

To add an event, copy ping.rs and give it your name eg "epic.rs"

then import epic.rs into mod.rs, then add it to the networkevent enum, then to the action for networkevent

you should have only created (and filled) one file, and modified mod.rs.


```
use serde::{Serialize, Deserialize};
use crate::handlers::{Handle, NetworkEvent};
use crate::connection::ConnectionLogic;

#[derive(Serialize, Deserialize, Debug)]
pub struct Epic {}
impl Handle for Epic {
    async fn action(&self, connection: &mut ConnectionLogic) {
        todo!()
    }
}

```

```
pub mod epic
...
pub enum NetworkEvent {
    Epic(epic::Epic)
}
...
NetworkEvent::Epic(epic) => epic.action(connection).await
```