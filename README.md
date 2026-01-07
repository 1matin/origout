# Origout: A Simple, Authoritative, P2P Git Collaboration System

## 1. Motivation

Modern Git collaboration is effectively centralized around platforms like GitHub and GitLab. While Git itself is decentralized, discovery, access control, moderation, and collaboration metadata are not. Existing decentralized alternatives (e.g., Radicle) attempt to remove authority entirely, but at the cost of significant complexity, poor moderation, and a mismatch with how teams actually work.

**Origout** (named as the opposite of "origin" in `git push origin master`, because there is no origin) is a **simpler, authoritative, cryptographically verifiable, peer-to-peer Git collaboration system** that:

* Removes reliance on centralized servers
* Preserves a single canonical repository state
* Supports moderation and role-based permissions
* Allows delegation and revocation of authority
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
* **P2P-first networking with Tor support**
* **Separation of code and collaboration metadata**
* **Hard forking as the ultimate escape hatch**
* **Seamless Git integration** via transparent CLI wrapper
* **Familiar web UI** running locally

Non-goals:

* Social consensus protocols
* Trust graphs or voting systems
* Immutable moderation-free history

---

## 3. Identity System

### 3.1 Decentralized Identifiers (DIDs)

Origout uses **DIDs** (Decentralized Identifiers) as the foundation for all identity:

* Each user and repository has a DID derived from their public key
* Format: `did:key:z6Mkf5rGMoatrSj1f9F2qv4SKHNLqe9qBE4gQ8vQ8qNXLqvZ`
* DIDs are permanent, cryptographic, and globally unique
* All delegation certificates and signatures reference DIDs

### 3.2 Human-Readable Handles

Users and repositories can claim **DNS-based handles** for readability:

* Verified via DNS TXT records (like Bluesky's AT Protocol)
* Examples: `@alice.dev`, `@rust-lang.org`, `@company.com`
* Domain owners automatically control their namespace
* Handles resolve to DIDs, which resolve to public keys

Handle resolution flow:

```bash
# User clones by handle
git clone @rust-lang/compiler.rust-lang.org

# Handle resolves to DID
@rust-lang.org → did:key:z6Mk...

# DID resolves to repo public key and RepoID
```

Handles are **for humans**, DIDs are **for the protocol**. If a handle changes, the DID remains constant, preserving all delegation chains and authority.

### 3.3 Repository Identity

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
* Delegation depth may be optionally limited

### 4.3 Social Bootstrap

New contributors gain capabilities through social trust:

* Community interaction (Discord, Matrix, forums, etc.)
* Repository owners set policies for unauthenticated access (e.g., `issue.create`)
* Trusted members delegate capabilities to new contributors
* Delegation reflects how real communities already build trust

This maps to existing project workflows where trust is earned through participation, not protocol mechanisms.

### 4.4 Repository Policy

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

### 5.3 Expiry and Renewal

Delegations may include expiry timestamps to limit blast radius and ensure periodic review.

---

## 6. Data Model

### 6.1 Code Storage

* Stored as a standard Git repository
* Canonical branches maintained by authorized keys
* Commits and ref updates are signed

### 6.2 Collaboration Metadata

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

### 6.3 Event Log

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

### 6.4 Conflict Resolution

When conflicting events occur (e.g., two moderators make incompatible decisions simultaneously):

1. The system enters a **confusion phase**
2. Both events are preserved
3. A person with sufficient privilege must manually resolve the conflict
4. Resolution is recorded as a new signed event

This only occurs when automatic resolution is impossible. The conflict becomes visible rather than silently resolved with last-write-wins.

---

## 7. Networking

### 7.1 Node Architecture

* **Every user is a node** (like Radicle)
* **Cache nodes** replace "seed nodes" for bootstrapping and availability

### 7.2 Cache Nodes

Cache nodes are configurable infrastructure nodes that:

* Store repositories temporarily based on activity
* **Delete inactive repositories** to prevent dead repos from consuming space
* Can be configured by owners with policies:
  - Manual approval vs. automatic acceptance
  - Repository size limits
  - Custom sidecar scripts for content validation
  - Activity thresholds for retention
* **Do not have authority** over repository content or governance
* Serve as availability/discovery helpers, not trusted parties

Repositories can be re-pushed to the network after deletion, but this requires manual action.

### 7.3 Transport

* libp2p (Go implementation)
* Encrypted by default
* Multiplexed streams
* **Standard Git protocol support**: nodes respond to traditional `git pull` requests for code only (without SQLite metadata)

### 7.4 Discovery

* Cache nodes for bootstrapping
* DHT or rendezvous-based discovery
* No authority assigned to cache nodes

### 7.5 Private Repositories

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
* **Easier than Tor**: no hidden services, onion routing, or network complexity

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

### 7.6 Tor Support

* Native onion-service support
* Optional Tor-only operation

---

## 8. Git Integration

### 8.1 Transparent CLI Wrapper

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

### 8.2 Local Web UI

Origout provides a **GitHub-like web interface running locally**:

* Launch with `git webui` (or `origout webui`)
* Serves on localhost (e.g., `http://localhost:3000`)
* Full issue tracking, code review, project management
* Familiar UX for users comfortable with GitHub/GitLab
* No need to reinvent UI paradigms

### 8.3 Official Mirrors

Origout supports traditional Git hosting for discoverability and transition:

* Nodes respond to standard Git pull requests (code only, no metadata)
* Projects can maintain GitHub/GitLab mirrors by:
  - Adding them as Git remotes: `git remote add github <url>`
  - Configuring the mirror to pull from a trusted origout node
* Owners can cryptographically sign endorsements of official mirrors
* Enables coexistence with centralized platforms during adoption

---

## 9. Validation Flow

When a node receives an action:

1. Verify signature
2. Resolve delegation chain to root
3. Check capability
4. Check revocation and expiry
5. Check repository policy

If validation fails, the action is ignored.

---

## 10. Forking

Any participant may hard-fork a repository by:

* Selecting a new root key
* Reusing or discarding prior history
* Obtaining a new RepoID

Forking is explicit and requires no protocol coordination. **Your fork, your repo—good luck promoting it.** This filters for legitimate community splits rather than creating abandoned-fork spam.

---

## 11. Security Properties

* No implicit trust
* Fully offline verifiable
* Resistant to spam and moderation abuse
* Explicit authority and accountability
* Minimal trusted computing base

---

## 12. Conclusion

Origout is a decentralized Git collaboration system that removes centralized infrastructure while preserving usability, moderation, and clarity of authority.

By combining:

* Git for code (with full backward compatibility)
* Cryptographic delegation for governance
* P2P transport with cache nodes for availability
* SQLite for queryable collaboration metadata
* Local web UI for familiar GitHub-like experience
* Transparent CLI wrapper for seamless integration
* Forking as the ultimate escape hatch

it offers a pragmatic alternative to both centralized platforms and overly complex decentralized designs.

The name "origout" reflects the core philosophy: there is no origin, no single point of control—only distributed authority, explicit trust, and practical engineering.
