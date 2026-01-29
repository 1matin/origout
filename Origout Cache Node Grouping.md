origout network had to be designed in a way that:

Supposing that all cache nodes have a fixed 100GB storage, and the network has 5 cache nodes; if cache nodes double in quantity, total network repo capacity has to be increased by, at least, 50%.

Not all cache nodes should seed any repository that is pushed to them. There should be a sync method like below:  
- Each cache node knows majority of other public cache nodes and repos they seed  
- There should be a function for a node to request cryptographic proof that a peer is indeed storing a certain repo, which takes sync delays in mind too (newer or older state, meaning that one of the two nodes engaging in this process are outdated compared to the other one)  
- This way, when a new repo push is sent to a cache node, it can calculate roughly that what percentage of public and permissive nodes are already seeding that suggested repo, and accepting to seed only if less than, e.g. 40% or 5 nodes were seeding that.  
- This mechanism has to be implemented in such a way that naturally groups cache nodes that have largely overlapping seeding lists, reducing sync latency as the number of middlemen in announcing an event is reduced.  
- However, we also have to keep in mind that this mechanism shouldn't risk data availability and reliability of the network. There aren't any "hardened" groups defined. Grouping has to be fluid and dynamic, to quickly respond to network disasters like a large portion of cache nodes going down.  