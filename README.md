# Origout: A Simple, Authoritative, P2P Git Collaboration System

## 1. Motivation

Modern Git collaboration is effectively centralized around platforms like GitHub and GitLab. While Git itself is decentralized, discovery, access control, moderation, and collaboration metadata are not. Existing decentralized alternatives (e.g., Radicle) attempt to remove authority entirely, but at the cost of significant complexity, poor moderation, and a mismatch with how teams actually work.

**Origout** (named as the opposite of "origin" in `git push origin master`, because there is no origin) is a **simpler, authoritative, cryptographically verifiable, peer-to-peer Git collaboration system** that:

* Removes reliance on centralized servers
* Preserves a single canonical repository state
* Supports moderation and role-based permissions
* Allows delegation and revocation of authority
* Includes sophisticated anti-spam and trust mechanisms
* Remains forkable and censorship-resistant
* Provides a familiar GitHub-like experience through local web UI

The design prioritizes **engineering practicality over ideological symmetry**.

---

## 2. Design Goals

* **Single authoritative repository state** per repository
* **Cryptographic verification of all actions**
* **Explicit ownership and governance**
* **Role- and capability-based permissions**
* **Delegation with transitive revocation**
* **Distributed trust and anti-spam system**
* **P2P-first networking with Tor support**
* **Separation of code and collaboration metadata**
* **Hard forking as the ultimate escape hatch**
* **Seamless Git integration** via transparent CLI wrapper
* **Familiar web UI** running locally

Non-goals:

* Social consensus protocols
* Global trust graphs or voting systems
* Immutable moderation-free history
* Dependence on centralized identity providers

---

## 3. Identity System

### 3.1 Decentralized Identifiers (DIDs)

Origout uses **DIDs** (Decentralized Identifiers) as the foundation for all identity:

* Each user and repository has a DID derived from their public key
* Format: `did:key:z6Mkf5rGMoatrSj1f9F2qv4SKHNLqe9qBE4gQ8vQ8qNXLqvZ`
* DIDs are permanent, cryptographic, and globally unique
* All delegation certificates and signatures reference DIDs

### 3.2 Human-Readable Handles

Users and repositories can claim **DNS-based handles** for readability, similar to Bluesky's AT Protocol:

* Verified via DNS TXT records
* Examples: `@alice.dev`, `@rust-lang.org`, `@company.com`
* Domain owners automatically control their namespace
* Handles resolve to DIDs, which resolve to public keys

**Root domains vs. subdomains**: Root domain handles (e.g., `@example.com`) provide higher initial trust signals than subdomains (e.g., `@user.example.com`) since acquiring domains requires financial investment. However, both can earn equal trust over time through legitimate behavior.

Handle resolution flow:

```bash
# User clones by handle
git clone @rust-lang/compiler.rust-lang.org

# Handle resolves to DID
@rust-lang.org → did:key:z6Mk...

# DID resolves to repo public key and RepoID
```

Handles are **for humans**, DIDs are **for the protocol**. If a handle changes, the DID remains constant, preserving all delegation chains and authority.

### 3.3 Email Verification (Optional)

Users can optionally verify email addresses to boost trust:

1. User signs a message: `"Hey, I'm <handle> and I'll sign this message for <node ID> so they verify this <email> belongs to me"`
2. User sends this signed message to a cache node with SMTP capabilities
3. The cache node validates the signature against the user's public key
4. If valid, the cache node propagates verification to the network
5. Other nodes independently verify the cryptographic proof

Email verification is:
* Optional but increases trust score
* Some repositories may require verified email for certain actions
* Verified without dependence on centralized email providers

### 3.4 Repository Identity

Each repository is identified by:

* A **root public key** (and corresponding DID)
* `RepoID = hash(root_public_key)`
* Optionally, a **DNS handle** (e.g., `@project.org`)

The holder of the root private key is the **owner** and can:

* Grant and revoke permissions
* Delegate authority
* Rotate keys or transfer ownership

---

## 4. Authority Model

### 4.1 Capabilities

Permissions are expressed as explicit, namespaced capabilities:

* `repo.view`
* `issue.create`
* `issue.moderate`
* `patch.create`
* `patch.review`
* `branch.write:main`
* `delegate.issue`

Capabilities are composable and scoped.

### 4.2 Delegation Certificates

Authority is granted via signed delegation certificates:

```
Delegation {
  issuer_key
  subject_key
  capabilities
  delegable_capabilities
  expiry (optional)
  signature
}
```

* The issuer must already possess the delegated capabilities
* Delegations form a tree rooted at the repository owner
* Delegation depth is configurable per-user and per-privilege

### 4.3 Rate-Limited Delegation (Invite Tree)

To prevent privilege escalation attacks and account compromise scenarios, delegation is rate-limited based on depth from the repository owner:

* **Ring 1** (owner): Can grant privileges to max 32 users per 10 days
* **Ring 2** (invited by owner): 16 users per 10 days per inviter
* **Ring 3** (invited by Ring 2): 8 users per 10 days per inviter
* **Ring 4** (invited by Ring 3): 4 users per 10 days per inviter
* **Ring 5** (invited by Ring 4): 2 users per 10 days per inviter
* **Ring 6+** (invited by Ring 5): Cannot grant privileges

These limits are:
* Per-user (each Ring 2 member can invite 16 users independently)
* Configurable by the repository owner
* Can be customized per-user or per-privilege type
* Maximum depth is owner-configurable (default 6)

This exponential decay prevents explosive spam from compromised accounts while allowing organic growth.

### 4.4 Social Bootstrap

New contributors gain capabilities through social trust:

* Community interaction (Discord, Matrix, forums, etc.)
* Repository owners set policies for unauthenticated access (e.g., `issue.create`)
* Trusted members delegate capabilities to new contributors
* Delegation reflects how real communities already build trust

This maps to existing project workflows where trust is earned through participation, not protocol mechanisms.

### 4.5 Repository Policy

Repository-wide policy defines what **unauthorized or unapproved users** may do.

Example policy:

```
unauthenticated:
  - repo.view
  - issue.create
```

Policies are signed by the repository owner and evaluated before delegation checks.

---

## 5. Revocation

### 5.1 Transitive Revocation

Delegations are valid **only while their issuer remains authorized**.

Revoking a delegation or key automatically invalidates all delegations issued by that key.

### 5.2 Explicit Revocation Records

Revocations are signed records:

```
Revocation {
  target (delegation_id or public_key)
  signature
}
```

Revocations propagate through the P2P network and are locally enforced.

### 5.3 Impact on Trust

When an authority bans a user from a repository or blocks them, the user's trust score decreases in nodes that receive this update. This creates network-wide consequences for bad behavior while maintaining decentralized enforcement.

### 5.4 Expiry and Renewal

Delegations may include expiry timestamps to limit blast radius and ensure periodic review.

---

## 6. Trust and Anti-Spam System

Origout implements a sophisticated, multi-layered anti-spam system that makes the network hostile to abuse while remaining frictionless for legitimate users.

### 6.1 Trust Score

Each node maintains a **local trust score** (0-100) for every peer it interacts with:

* **Purely local**: Each node calculates trust independently with no consensus
* **Zero-trust between nodes**: Nodes never believe claims from other nodes unless cryptographically verifiable
* **Not globally visible**: Users cannot see their trust score; it's internal node state
* **New users start at 60**: Optimized for legitimate use cases, not paranoia

Trust scores gradually increase as nodes observe legitimate behavior over time. High-trust users experience:
* Little to no Proof of Work challenges
* Higher storage priority on cache nodes
* Faster connection acceptance
* Greater rate limits

### 6.2 Trust Signals

Multiple factors influence trust calculation (exact formula to be defined in implementation):

* **Time**: Long-term legitimate users build trust naturally
* **Email verification**: Verified emails provide moderate trust boost
* **DNS handle type**: Root domains provide higher initial trust than subdomains
* **Repository activity**: Consistent, legitimate contributions increase trust
* **Bans and blocks**: Receiving bans decreases trust across the network
* **Proof of Misbehavior (PoM)**: Verified misbehavior reports significantly decrease trust
* **Connection behavior**: Stable, non-abusive connection patterns
* **Delegation depth**: Users closer to repository owners may have higher trust

### 6.3 Proof of Work (PoW)

When peers connect or initiate communication (e.g., pushing a repository), the receiving node may request a **Proof of Work challenge**:

* **Dynamic difficulty** based on:
  - The connecting peer's trust score (lower trust = harder PoW)
  - The receiving node's current load (higher load = harder PoW)
* **High-trust users bypass PoW** entirely under normal conditions
* **Max difficulty limits**: Users can set maximum acceptable PoW difficulty (default ~10 minutes for users, lower for cache nodes)
* **Per-connection challenges**: Each connection requires separate PoW, making mass spam computationally infeasible

### 6.4 Proof of Misbehavior (PoM)

When a node detects abusive behavior, it can generate a cryptographically verifiable **Proof of Misbehavior**:

**PoM Creation**: Since all actions are signed, nodes can package evidence:
* Signed actions (e.g., 1000 spam issues with timestamps)
* User's public key
* Contextual metadata (timestamps, repository state)

**PoM Propagation**: PoMs are propagated through the network (via DHT/gossip protocol, details to be finalized)

**PoM Verification**: Each receiving node:
1. Independently verifies the cryptographic signatures
2. Validates the behavior matches local abuse criteria
3. Adjusts the offending peer's local trust score accordingly

This creates network-wide reputation consequences without requiring trust between nodes.

### 6.5 Spam Detection Heuristics

Nodes automatically detect and respond to suspicious behavior:

**Mass Repository Creation**:
* Track count of new repositories pushed by each user
* Rate-limit users who exceed thresholds
* Generate PoM for extreme violations

**Mass Content Creation**:
* Detect large batches of issues/comments/patches
* Example: 1000 issues created by non-maintainers triggers PoM
* Owner bulk imports (e.g., GitHub migrations) are exempt

**Sudden Activity Spikes**:
* Track baseline activity patterns
* Flag sudden deviations (e.g., silent user suddenly creating 500 issues)

When spam is detected:
1. Node rejects the action
2. Generates PoM with cryptographic proof
3. Propagates PoM to network
4. Decreases local trust score for the offender

### 6.6 Trust Recovery

Low-trust users can recover trust over time:

* **Recovery timeline**: Approximately 2 weeks for full recovery through legitimate behavior
* **Recovery cap**: Users can naturally recover to trust score 50
* **Above 50**: Requires active trust signals (time, contributions, verifications)
* **Not permanent bans**: Low trust increases friction (more PoW) but doesn't prevent participation

This prevents permanent ostracization while maintaining consequences for bad behavior.

### 6.7 Scriptable Validation

Nodes can optionally implement custom validation logic:

* **Default suite**: All nodes ship with the full anti-spam system described above
* **Custom extensions**: Nodes can add per-node validation scripts
* **LLM integration**: Nodes can optionally use language models for content analysis
* **Purely local**: Custom validation only affects what the node stores locally
* **No network impact**: Other nodes are unaffected by custom validation rules

This allows experimentation and specialized filtering without fragmenting the network.

---

## 7. Data Model

### 7.1 Code Storage

* Stored as a standard Git repository
* Canonical branches maintained by authorized keys
* Commits and ref updates are signed

### 7.2 Collaboration Metadata

Non-code collaboration data is **not stored in Git**:

* Issues
* Comments
* Reactions
* Code reviews
* Projects
* Moderation actions

Instead:

* Actions are recorded as signed events
* Events are replicated via P2P
* Each node materializes state into SQLite

This avoids Git bloat and enables moderation and querying.

### 7.3 Event Log

Each collaboration action is represented as a signed event:

```
Event {
  repo_id
  scope
  author_key
  payload
  timestamp
  signature
}
```

Events are:

* Append-only
* Verifiable offline
* Prunable via compaction

**Metadata retention**: Repository owners define retention policies for event logs. When repositories grow too large (e.g., exceeding 1GB), nodes may refuse new pushes, requiring owners to prune old metadata or increase limits.

### 7.4 Conflict Resolution

When conflicting events occur (e.g., two moderators make incompatible decisions simultaneously):

1. The system attempts **automatic resolution** using predefined rules
2. If automatic resolution fails, both events are preserved (confusion phase)
3. A person with sufficient privilege must manually resolve the conflict
4. Resolution is recorded as a new signed event

For **Git codebase conflicts**, standard Git merge conflict resolution applies (handled for decades). For **origout-specific data** (SQLite databases, metadata), the system attempts automatic resolution first, falling back to human intervention when necessary.

This only occurs when automatic resolution is impossible. The conflict becomes visible rather than silently resolved with last-write-wins.

---

## 8. Networking

### 8.1 Node Architecture

* **Every user is a node**: Users run origout locally, participating directly in the P2P network
* **Cache nodes**: Infrastructure nodes that improve availability and bootstrapping
* **No privileged nodes**: Cache nodes have no authority over repository content or governance

### 8.2 Cache Nodes

Cache nodes are configurable infrastructure nodes that:

* **Temporary storage**: Store repositories based on activity, not permanently
* **Automatic cleanup**: Delete inactive repositories to prevent storage bloat
* **Intelligent replication**: Cache nodes coordinate to optimize network-wide capacity utilization
* **Configurable policies**:
  - Manual approval vs. automatic acceptance of repositories
  - Repository size limits (individually configurable, e.g., 1GB default)
  - Custom sidecar scripts for content validation
  - Activity thresholds for retention
* **Trust-based prioritization**: High-trust users' repositories remain cached longer
* **Repository pinning**: Nodes can pin specific repositories to prevent removal even when storage is full
* **No authority**: Cannot modify repository content or governance
* **Re-pushable**: Deleted repositories can be re-pushed by users when needed

**Running a cache node**: Anyone can run a cache node by hosting the origout binary. No special configuration or compensation model is required—operators choose to provide availability as infrastructure.

#### Cache Network Capacity Scaling

The cache node network is designed so that adding nodes meaningfully increases total capacity:

**Design goal**: If all cache nodes have 100GB storage and there are 5 nodes, doubling to 10 nodes should increase total network capacity by at least 50%.

**Replication strategy**:
* Cache nodes track what other public cache nodes are seeding
* When receiving a new repository push, nodes calculate what percentage of the network already has it
* Nodes accept the repository only if fewer than ~40% of cache nodes (or 5 nodes, whichever is less) are seeding it
* This prevents over-replication while ensuring adequate availability

**Proof of Storage (PoS)**:
* Nodes announce their inventory via signed messages
* Other nodes can challenge these claims based on trust score
* Challenger solves a PoW to initiate challenge
* Challenger picks a repository it also has and issues a cryptographic challenge
* If the challenged node fails to provide correct proof, the challenger has a signed **Proof of Misbehaviour (PoM)**
* PoM is broadcast network-wide, decreasing the liar's trust score everywhere

**Dynamic clustering**:
* Cache nodes naturally form fluid clusters based on overlapping seed lists
* Clustering reduces sync latency by minimizing hops for event propagation
* Groupings are not hardened—they adapt dynamically to network changes
* When cache nodes go offline, remaining nodes automatically rebalance

This design ensures that network capacity scales efficiently with the number of cache nodes while maintaining data availability and preventing storage waste.

Cache nodes serve as availability and discovery helpers, treating storage as temporary cache rather than permanent archive.

### 8.3 Transport

* **libp2p** (Go implementation)
* Encrypted by default
* Multiplexed streams
* **Standard Git protocol support**: Nodes respond to traditional `git pull` requests for code only (without SQLite metadata)

### 8.4 Discovery

* Cache nodes for bootstrapping
* DHT or rendezvous-based discovery
* No authority assigned to cache nodes

### 8.5 Connection Management and Bandwidth

When peers (users or cache nodes) attempt connections:

* Each node individually tracks peer behavior
* PoW difficulty adjusts per-connection based on trust and load
* **Rate limiting**: Combined with PoW and trust to prevent bandwidth exhaustion
* **Trust-based bandwidth allocation**: Low-trust users may face stricter rate limits on large fetches
* Prevents mass connection spam through computational cost
* High-trust peers connect with minimal friction

**Note on DDoS**: Massive network-level DDoS attacks (overwhelming packet floods) are outside the scope of software-level solutions and would require infrastructure-level protection (e.g., CDN, filtering).

### 8.6 Private Repositories

Origout supports **end-to-end encrypted (E2EE) private repositories** that can be stored on public cache nodes without revealing content.

#### Encryption Architecture

* Based on **Signal Protocol** principles (Double Ratchet Algorithm)
* **Forward secrecy** for Git objects: even if keys are compromised later, past content remains encrypted
* **Separate encryption schemes** for:
  - Git objects (commits, trees, blobs)
  - SQLite collaboration metadata (issues, comments, reviews)

#### Key Distribution

* Repository owner generates a **repository encryption key**
* Keys are distributed only to authorized collaborators via:
  - Out-of-band secure channels (Signal, PGP, in-person)
  - Or encrypted key exchange using collaborators' public keys
* Each collaborator's capability delegation can include encrypted key material

#### Storage on Public Nodes

* Encrypted repositories can be stored on **public cache nodes**
* Cache nodes see only:
  - Encrypted blobs (indistinguishable from random data)
  - Repository metadata (size, activity, RepoID)
* No cache node can decrypt content without authorization
* **Simpler than Tor**: No hidden services, onion routing, or network complexity required

#### Ratcheting for Forward Secrecy

* Git objects use **per-commit key derivation**
* Each commit advances the ratchet, generating new encryption keys
* Compromising the current key doesn't reveal historical commits
* SQLite events use **periodic key rotation** to limit exposure

#### Tradeoffs

**Advantages over Tor-only private repos:**
* Simpler infrastructure (no hidden services)
* Can use fast public cache nodes
* Better performance and availability
* Easier to integrate with existing networks

**Limitations:**
* Metadata still visible (repo size, activity patterns, contributor count)
* Requires out-of-band key distribution for initial access
* More complex than plaintext repos

Private repositories remain **opt-in**. Public repositories use no encryption for simplicity and efficiency.

### 8.7 Tor Support

* Native onion-service support for additional privacy
* Optional Tor-only operation for maximum anonymity

---

## 9. Git Integration

### 9.1 Transparent CLI Wrapper

Origout is implemented as a **transparent wrapper for Git**:

* Respects all Git reserved commands
* Can be aliased so `git` calls invoke `origout` everywhere
* Passes unknown commands directly to Git
* Adds origout-specific commands (e.g., `git webui`, `git clone <repo-id>`)

Example usage:

```bash
git clone <repo-id>           # origout handles P2P clone
git commit -m "fix"           # passed to git normally
git push                      # origout adds P2P hooks
git webui                     # origout-specific: launch local UI
git log                       # passed to git normally
```

This design allows users to keep their Git muscle memory while origout handles P2P coordination transparently.

### 9.2 Distribution Model

Origout uses a **Git-like distribution model** with pseudo-subcommands:

* **Main binary**: `origout` - wrapper that dispatches to specialized tools
* **Specialized binaries**: 
  - `origout-node` - P2P server and node management (called via `origout node ...`)
  - `origout-wui` - local web UI server (called via `origout webui ...`)
  - `origout-node-index` - node index generator for bootstrapping
  - Additional tools as needed

When `origout` encounters an unknown command, it attempts to execute `origout-<command>` as a separate binary, similar to how `git` works with `git-<command>`.

**Universal JSON output**: All origout commands accept a `--json` flag that returns consistent, versioned JSON output. This enables:
* Easy testing and automation
* Third-party tool development
* Alternative UIs and integrations
* Scriptable workflows

Example:
```bash
origout node status --json
{
  "version": "1.0",
  "repos_seeded": 47,
  "storage_used": "73GB",
  "peer_connections": 23
}
```

### 9.3 Local Web UI

Origout provides a **GitHub-like web interface running locally**:

* Launch with `origout webui` (or `git webui` via wrapper)
* Serves on localhost (e.g., `http://localhost:3000`)
* **Technology stack**: Go + Fiber + Templ + GORM GEN
* **Not for public hosting**: Automatically uses the cryptographic keys of the machine it runs on
* **Progressive enhancement**: Fully functional without JavaScript, enhanced with JS for convenience (reactions, comments without page refresh)
* **Familiar UX**: UI philosophy between SourceHut and GitHub—simpler than GitHub, more user-friendly than SourceHut
* **First-class theming**: Dark mode and high-contrast themes as core features
* Full issue tracking, code review, project management

The web UI is architecturally a **wrapper around the CLI**, leveraging the `--json` flag for all data operations. This keeps business logic in one place and ensures consistency.

### 9.4 Official Mirrors

Origout supports traditional Git hosting for discoverability and transition:

* Nodes respond to standard Git pull requests (code only, no metadata)
* Projects can maintain GitHub/GitLab mirrors by:
  - Adding them as Git remotes: `git remote add github <url>`
  - Configuring the mirror to pull from a trusted origout node
* Owners can cryptographically sign endorsements of official mirrors
* Enables coexistence with centralized platforms during adoption

**Migration from centralized platforms**: Origout will provide tooling to import existing repositories with full history, issues, and pull requests from platforms like GitHub. Since these activities were performed on GitHub (not origout), they lack cryptographic signatures. The migration process must handle unsigned historical data appropriately—implementation details to be defined.

---

## 10. Validation Flow

When a node receives an action:

1. **Trust check**: Evaluate peer's local trust score
2. **PoW challenge** (if needed): Request and verify Proof of Work
3. **Signature verification**: Verify cryptographic signature
4. **Delegation chain resolution**: Trace delegation to repository root
5. **Capability check**: Verify action is within delegated capabilities
6. **Revocation check**: Ensure delegation hasn't been revoked
7. **Expiry check**: Verify delegation hasn't expired
8. **Repository policy check**: Validate against repository-wide policy
9. **Spam heuristics**: Check for mass creation, suspicious patterns
10. **Custom validation** (if configured): Run node-specific validation scripts

If validation fails at any step, the action is rejected.

---

## 11. Bootstrap and Version Enforcement

### 11.1 Initial Repository Creation

To create a new repository:

1. **Bootstrap**: Run `origout bootstrap` (or `git bootstrap` via wrapper) to initialize local database files and metadata
2. **Join network**: Connect to the P2P network (required for pushing)
3. **Push**: Push the repository to the network for the first time

Repositories exist independently from the network, but network connectivity is required to share them. Users can work on repositories locally before pushing.

### 11.2 Network Discovery and Bootstrap

Origout includes a sophisticated bootstrap mechanism to ensure reliable network connectivity:

**Node Index System**:
* **Node indexes** are lightweight JSON HTTP endpoints listing discovered nodes
* Can be hosted on static file hosting (GitHub, S3, personal servers)
* Generated by `origout-node-index` binary
* Self-updating: Periodically connects to listed nodes, updates the list, commits changes

**Hardcoded diverse list**: The binary ships with a comprehensive list of node indexes from multiple sources:
* Official origout node indexes
* Community-run indexes
* Individual cache nodes
* Reduces dependency on any single infrastructure provider

Example node index structure:
```json
{
  "version": "1.0",
  "last_updated": "2026-01-29T12:00:00Z",
  "nodes": [
    {"id": "...", "address": "..."},
    {"id": "...", "address": "..."}
  ]
}
```

This approach ensures that:
* New users can always discover the network
* No single point of failure for bootstrap
* Indexes can be updated independently
* Community can contribute discovery infrastructure

### 11.3 Repository Caching and Subscription

Users control which repositories they actively track:

* **Cache/Seed**: Users can cache (seed) repositories, similar to Radicle's seeding
* **Subscribe to updates**: Cached repositories automatically receive updates from other nodes
* **Local control**: Each user decides how many repositories to cache based on their resources
* **Not GitHub-style feeds**: Discovery mechanisms exist but without centralized feeds

### 11.4 Repository Reporting and Moderation

Origout includes a distributed reporting system for repositories (not individual issues or activities within them):

**Report categories**:
1. Adult content
2. Copyright infringement
3. Owner inactivity (repository isn't being moderated)
4. Hate, discrimination, terrorism
5. Other illegal activity

**Reporting workflow**:
1. Users report repositories to the network
2. Volunteer moderators access reports via `origout report moderate`
3. Moderators see:
   - Repository ID and clone command
   - Report count and most common reason
   - Trust index of reporters
   - Current vote distribution from other moderators
4. Moderators cast votes on whether reports are valid
5. Network-wide consensus emerges from volunteer moderation

**Owner inactivity handling**:
* First strike: Warning
* Second strike: Repository becomes read-only until owner takes action
* Third strike: Repository becomes read-only permanently (to be discussed further)

Example moderation interface:
```
Showing list of repositories with active reports
Your trust score is estimated: Mid-high

Repo #1: RID
Report count: 5
Most common reason: Copyright
Trust index of reporters: Mid-low
60% of volunteer moderators approved
30% refused
10% had no opinions
Total voted moderators: 69

Clone the repo with: origout clone <RID>
Cast your vote with: origout report vote <RID> [approve|refuse|abstain]
```

This system provides **decentralized moderation** without centralized authority, relying on volunteer consensus and trust scores.

### 11.5 Cross-Repository References

Issues and other metadata can reference other repositories:

* Links can be created to external repositories or issues
* **No verification**: The system does not verify that referenced repositories or issues exist
* References are informational, not enforced by protocol

### 11.6 Key Management

Origout uses **self-custody** for private keys:

* Users are responsible for securely storing their private keys
* **No recovery mechanism**: Lost keys mean lost repository ownership
* Users should implement their own backup strategies (encrypted backups, hardware keys, etc.)
* Key rotation is possible while maintaining the same DID

### 11.7 Pre-1.0 Version Enforcement

Before version 1.0.0, origout includes a version enforcement mechanism:

* **Official bootstrap node only**: Client uses only the official node initially, even if it discovers others
* **Mandatory updates**: Users are forced to update to the latest version
* **Prevents fragmentation**: Ensures network stability during protocol development
* **Removed at 1.0.0**: After protocol stabilizes, version enforcement is removed

This pragmatic approach prioritizes stability over decentralization purity during early development.

---

## 12. Forking

Any participant may hard-fork a repository by:

* Selecting a new root key
* Reusing or discarding prior history
* Obtaining a new RepoID

Forking is explicit and requires no protocol coordination. **Your fork, your repo—good luck promoting it.** This filters for legitimate community splits rather than creating abandoned-fork spam.

---

## 13. Security Properties

* **No implicit trust**: All actions require cryptographic verification
* **Fully offline verifiable**: Delegation chains and signatures can be validated without network access
* **Resistant to spam**: Multi-layered anti-spam system makes abuse computationally expensive
* **Moderation with accountability**: Authority is explicit and actions are signed
* **Sybil-resistant**: Trust is earned slowly; mass identity creation provides no advantage
* **Self-custody**: Users maintain full control of their private keys (and responsibility for securing them)
* **Minimal trusted computing base**: Only trust your own node and cryptographic primitives
* **Graceful degradation**: Low trust increases friction but doesn't prevent participation

## 14. Platform Scope

Origout is designed as a **desktop-first** system:

* **Primary target**: Linux, macOS, and Windows desktop environments
* **Mobile**: No native mobile support planned
* **Remote access**: Users can connect to their desktop node or a personal Raspberry Pi node from mobile devices via remote access
* **Resource requirements**: Assumes sufficient disk, memory, and bandwidth for running a full node

This focuses development resources on the core P2P experience rather than attempting cross-platform parity.

---

## 15. Comparison to Radicle

| Aspect               | Radicle           | Origout                  |
| -------------------- | ----------------- | ------------------------ |
| Authority            | None              | Explicit owner           |
| Canonical state      | No                | Yes                      |
| Moderation           | Impossible        | First-class              |
| Permissions          | Implicit          | Capability-based         |
| Revocation           | Weak              | Transitive               |
| Anti-spam            | Basic             | Multi-layered (PoW/PoM)  |
| Trust model          | Social graphs     | Local, cryptographic     |
| Data storage         | Git-only          | Git + SQLite             |
| Infrastructure       | Seed nodes        | Cache nodes              |
| Repository caching   | Seeding           | Caching/subscription     |
| Git integration      | Separate tools    | Transparent wrapper      |
| Web UI               | Separate          | Local, built-in          |
| Mobile support       | Limited           | None (desktop-focused)   |
| GitHub migration     | Manual            | Planned tooling          |

---

## 16. Comparison to GitHub

| Aspect                | GitHub             | Origout                      |
| --------------------- | ------------------ | ---------------------------- |
| Infrastructure        | Centralized        | P2P + cache nodes            |
| Authority             | Platform-mediated  | Cryptographic                |
| Data ownership        | GitHub Inc.        | Repository owner             |
| Access control        | Platform policy    | Owner-defined capability     |
| Anti-spam             | Centralized        | Distributed (PoW/PoM/Trust)  |
| Discoverability       | Central registry   | RepoID + cache nodes + DNS   |
| UI/UX                 | Web-based          | Local web UI                 |
| Git compatibility     | Full               | Full                         |
| Fork model            | Low friction       | Explicit, high friction      |
| Censorship resistance | None               | High                         |
| Email verification    | Required           | Optional, decentralized      |

---

## 16. Conclusion

Origout is a decentralized Git collaboration system that removes centralized infrastructure while preserving usability, moderation, and clarity of authority.

By combining:

* **Git for code** (with full backward compatibility)
* **Cryptographic delegation** for governance
* **Sophisticated anti-spam** (PoW, PoM, local trust scores)
* **P2P transport** with cache nodes for availability
* **SQLite** for queryable collaboration metadata
* **Local web UI** for familiar GitHub-like experience
* **Transparent CLI wrapper** for seamless integration
* **Forking** as the ultimate escape hatch
* **Practical security** optimized for legitimate users

it offers a pragmatic alternative to both centralized platforms and overly complex decentralized designs.

The name "origout" reflects the core philosophy: there is no origin, no single point of control—only distributed authority, explicit trust, practical anti-spam, and sound engineering.
