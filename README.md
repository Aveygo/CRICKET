> [!WARNING]  
> cricket is still in active development, there are no public bootstrap nodes for the time being
>
> Also the code is very messy so prepare your r/eyebleach before continuing

# Legal

cricket was developed purely for academic purposes to explore the feasibility of an experimental protocol. **cricket is not a service, nor does it provide one.** That being said, each user is expected to comply with the laws within their respective jurisdictions.

## Pre-rant

Most 'decentralized' platforms like: Mastodon, Lemmy, Bluesky, Matrix, Nostr and a thousand others are actually not-very-secretly federated. This is because it solves a lot of problems that p2p systems have, however, they normally accidentally end up being heavily centralized around their main instance. Even blockchain technology is still non-ideal, which requires users to spend money to use their service.

The closest we have is Scuttlebutt, which still relies on self-hosted rooms (although for good reason I would add) which is extremely infuriating as it really is only step away from being a complete solution. Instead, I wanted a **true** decentralized protocol that is imperfect but good enough. 

This is my solution in a bucket of a thousand (a true 927 moment), but I hope that it serves its purpose and grows into something I could be proud of, or to at least inspire something better.

# Protocol

1. Alice will receive a post from Bob via a bootstrap node.
2. If Alice believes that Bob is an acceptable peer, she will send a 'trust' request.
3. Bob accepts the request if Alice is similarly acceptable. 
4. The process repeats where Alice eventually finds her closest peers, and the bootstrap nodes are no longer needed.

TLDR: A friend of my friend is also my friend

By restricting how the users connect to one another, each node becomes a moderator for their peers. Unfortunately this does mean that everyone has a role to play in managing the worst of humanity, but by building the network and only connecting to trusted peers, this issue is at least partially solved the more the network matures. 

# The Elephant in the Room

To deal with NAT traversal, this project uses Iroh's [public relays](https://www.iroh.computer/docs/concepts/relay).

This is the weakest link in the chain, and means a couple of things:
1. They theoretically have the ability to reject nodes from joining the network
2. If the relays go down, then no one can make any **new** connections with unknown peers

Currently if Iroh misbehaves, the network will still be (mostly) functional and the relays can be self-hosted in a pinch, which is why I still consider cricket to be fully decentralized. 

cricket was always going to be an imperfect solution, but a solution nonetheless. Maybe a relay-less fix lies in tor? 🤔 😉



# What is guaranteed

1. Self-owned social media; full control your own recommendation algorithm with no 3rd party to influence what you see or don't see.

2. Extreme resilience; if there is even at least one copy of a post on the network, it can be replicated to everyone in seconds

3. End-to-end encryption to your trusted peers

4. No large easy targets for hackers or foreign governments to manipulate or take down.

5. Scalability - reach an audience of your 10 friends, or to thousands of viewers.

# Legal

It's a strange experience having to deal with ethical challenges while trying to make a performant peer to peer protocol, and that's why I wanted to spend a moment to explain my situation and how I'm (trying to) approach it.

The core issue is that most countries consider social media sites under the jurisdiction of their laws, so what happens if the people are the social media? I think this problem is too much of a 'we live a society' type moment for my programming brain to handle, and because existing laws don't really consider it, it feels like making an individual decision would be too 'immature' for the impact it can have.

That's why for the time being, I would rather have this problem solved 'together' when it becomes a real issue in the first place, rather than as a theoretical (yet still entertaining) scenario.




