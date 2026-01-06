# Proposal: A Simple, Authoritative, P2P Git Collaboration System

## 1. Motivation

Modern Git collaboration is effectively centralized around platforms such as GitHub and GitLab. While Git itself is decentralized, discovery, access control, moderation, and collaboration metadata are not. Existing decentralized alternatives (e.g., Radicle) attempt to remove authority entirely, but at the cost of significant complexity, poor moderation, and a mismatch with how teams actually work.

This proposal describes a **simpler, authoritative, cryptographically verifiable, peer-to-peer Git collaboration system** that:

* Removes reliance on centralized servers
* Preserves a single canonical repository state
* Supports moderation and role-based permissions
* Allows delegation and revocation of authority
* Remains forkable and censorship-resistant

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

Non-goals:

* Social consensus protocols
* Trust graphs or voting systems
* Immutable moderation-free history

---

## 3. Repository Identity

Each repository is identified by a **root public key**:

* `RepoID = hash(root_public_key)`
* The holder of the root private key is the **owner**

The root key:

* Grants and revokes permissions
* Can delegate authority
* Can rotate itself or transfer ownership

---

## 4. Authority Model

### 4.1 Capabilities

Permissions are expressed as explicit, namespaced capabilities:

Examples:

* `repo.view`
* `issue.create`
* `issue.moderate`
* `patch.create`
* `patch.review`
* `branch.write:main`
* `delegate.issue`

Capabilities are composable and scoped.

---

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

---

### 4.3 Invite Trees

Delegations may grant the right to further delegate specific capabilities.

Example:

* Owner delegates `issue.create` and `delegate.issue` to user A
* User A delegates `issue.create` to user B

This forms an invite tree remembered implicitly through delegation chains.

---

## 5. Revocation

### 5.1 Transitive Revocation

Delegations are valid **only while their issuer remains authorized**.

Revoking a delegation or key automatically invalidates all delegations issued by that key.

---

### 5.2 Explicit Revocation Records

Revocations are signed records:

```
Revocation {
  target (delegation_id or public_key)
  signature
}
```

Revocations propagate through the P2P network and are locally enforced.

---

### 5.3 Expiry and Renewal

Delegations may include expiry timestamps to limit blast radius and ensure periodic review.

---

## 6. Repository Policy

Repository-wide policy defines what **unauthorized or unapproved users** may do.

Example policy:

```
unauthenticated:
  - repo.view
  - issue.create
```

Policies are signed by the repository owner and evaluated before delegation checks.

---

## 7. Data Model

### 7.1 Code

* Stored as a standard Git repository
* Canonical branches maintained by authorized keys
* Commits and ref updates are signed

---

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

---

## 8. Event Log

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

---

## 9. Networking

### 9.1 Transport

* libp2p (Go implementation)
* Encrypted by default
* Multiplexed streams

---

### 9.2 Discovery

* Seed nodes for bootstrapping
* DHT or rendezvous-based discovery
* No authority assigned to seed nodes

---

### 9.3 Tor Support

* Native onion-service support
* Optional Tor-only operation

---

## 10. Validation Flow

When a node receives an action:

1. Verify signature
2. Resolve delegation chain to root
3. Check capability
4. Check revocation and expiry
5. Check repository policy

If validation fails, the action is ignored.

---

## 11. Forking

Any participant may hard-fork a repository by:

* Selecting a new root key
* Reusing or discarding prior history

Forking is explicit and requires no protocol coordination.

---

## 12. Security Properties

* No implicit trust
* Fully offline verifiable
* Resistant to spam and moderation abuse
* Explicit authority and accountability
* Minimal trusted computing base

---

## 13. Comparison to Radicle

| Aspect          | Radicle    | This Proposal    |
| --------------- | ---------- | ---------------- |
| Authority       | None       | Explicit owner   |
| Canonical state | No         | Yes              |
| Moderation      | Impossible | First-class      |
| Permissions     | Implicit   | Capability-based |
| Revocation      | Weak       | Transitive       |
| Data storage    | Git-only   | Git + SQLite     |

---

## 14. Conclusion

This proposal describes a decentralized Git collaboration system that removes centralized infrastructure while preserving usability, moderation, and clarity of authority.

By combining:

* Git for code
* Cryptographic delegation for governance
* P2P transport for distribution
* Forking as the ultimate escape hatch

it offers a pragmatic alternative to both centralized platforms and overly complex decentralized designs.
