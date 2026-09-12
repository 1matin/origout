> [!WARNING]
> This document is an aggregate of +20 drafts written by me.
> Although 100% of the ideas (with no exception) source inside my brain, this whitepaper itself is written by AI.
> Origout is my "no-AI, stay human" project, and we don't accept vibecoded contributions for **code** anyway.

# Origout: A Decentralized, Authoritative Git Collaboration Protocol

## 1. Abstract / Introduction

Git itself is decentralized, but the infrastructure teams build around it — discovery, access control, moderation, issue
tracking, code review — is not. Platforms like GitHub and GitLab centralize all of this, even though the version control
system underneath has no such requirement.

Existing attempts to decentralize this layer (Radicle, for instance) often remove authority along with infrastructure.
The result is a network with no clear owner for a repository, weak moderation, and a workflow that doesn't match how
real teams actually operate — someone is always ultimately responsible for a project.

Origout takes a different position: decentralize *where a repository lives*, not *who controls it*. Every repository has
a single, cryptographic owner. Distribution, discovery, and collaboration metadata run over a peer-to-peer network, but
that network never has a say in who's allowed to do what.

The result should look, to a user, almost exactly like Git:

```
git clone og://example.com/project
```

Everything else — peer discovery, replication, proof-of-work admission, DNS resolution, DHT fallback — happens
underneath that command. The user doesn't need to understand any of it to use it.

Origout is not a small system, and this document won't pretend otherwise. What's simple is the experience of using it —
not the protocol behind it. That's a deliberate tradeoff: pragmatic decentralization, invisible P2P infrastructure,
cryptographic authority, local enforcement, and full interoperability with Git, at the cost of real implementation
complexity.

## 2. Design Goals and Non-Goals

**Goals**

- A single, cryptographically authoritative owner per repository
- Cryptographic verification of every action
- Capability-based permissions with delegation and transitive revocation
- Local, resource-based anti-abuse (proof-of-work and per-peer history) instead of a propagated reputation system
- P2P-first networking, with Tor support
- Clear separation between code (Git objects) and collaboration metadata (issues, reviews, etc.)
- Hard forking as an unconditional escape hatch
- A transparent Git CLI wrapper — existing muscle memory keeps working
- A familiar, GitHub-like local web UI
- P2P mechanics that stay invisible behind ordinary Git commands

**Non-goals**

- Global trust graphs, reputation scores, or voting systems
- Social-consensus protocols
- Guaranteed Sybil immunity
- Immutable, moderation-free history
- Dependence on centralized identity providers

**A note on private repositories.** End-to-end encrypted private repositories are a planned future phase and are not
specified here. This document describes public repositories only.

## 3. Architecture Overview

Origout rests on three concepts that are easy to conflate but need to stay separate:

| Concept              | Question it answers                                  | Basis                                              |
|----------------------|------------------------------------------------------|----------------------------------------------------|
| **Identity**         | Who cryptographically signed this?                   | Public keys, bound to human-readable names via DNS |
| **Authority**        | Is this identity allowed to do this?                 | Repository ownership, capabilities, delegation     |
| **Peer familiarity** | How much friction should my node apply to this peer? | Local, private, per-node observation               |

Identity is global and portable. Authority is scoped to a specific repository and derived entirely from cryptographic
delegation — never from reputation, DNS prestige, or history. Peer familiarity is local: it only ever affects how much
proof-of-work *your own node* asks a peer to do, and it's never shared, gossiped, or treated as a protocol-level claim
about anyone.

```
                         DNS
                          |
              +-----------+-----------+
              |                       |
        User / Node Identity     Repository Address
              |                       |
          DNS TXT -> Key         DNS -> RepoID
              |                       |
              +-----------+-----------+
                          |
                       Origout
                          |
        +-----------------+-----------------+
        |                 |                 |
     Identity          Authority        Anti-Abuse
        |                 |                 |
   public keys        capabilities         PoW
   signatures         delegation      local peer history
                       revocation
                          |
                     Repository
                          |
             +------------+------------+
             |            |            |
            Git         SQLite        LFS
          objects    Issues / PRs    objects
```

The UX principle this architecture is built to support:

> "It's Git, except the repository doesn't belong to a website."

## 4. Identity and DNS

### 4.1 Cryptographic identity

Every user and every node has a cryptographic identity based on a public/private key pair. All signatures, delegations,
and revocations reference the public key directly — there's no intermediate abstraction layer between "identity" and
"key."

### 4.2 DNS as a naming layer

A public key is not memorable, so a user may bind a human-readable DNS name to it via a signed TXT record:

```
alice.example.com
        |
        v
    DNS TXT
        |
        v
  Alice's public key
```

DNS gives Origout a way to discover and display readable names. It is **not** part of the identity itself, and a DNS
compromise does not automatically grant authority over the corresponding key — the client still checks that anything
presented under that name is validly signed. On first resolution a client caches the name-to-key binding; any later
change to that binding surfaces a prominent warning rather than being silently trusted.

### 4.3 Node identity

There's no special "anonymous infrastructure" category for nodes. A node is a peer in the same identity model as a
user — a piece of software participating in the network, often but not always run by a person who also holds a user
identity. Origout uses **peer** as the generic protocol term, **user** for a human or account, and **node** for the
running software instance. A cache node (§10) is also just a node, with a particular operating policy.

### 4.4 Hosted and free identities

Not everyone owns a domain. Origout retains the option of hosted, free DNS identities for onboarding — a convenience
layer, not a requirement. Owning a root domain is never a prerequisite for participating in Origout.

### 4.5 Root domains and initial admission cost

A root-domain identity (`alice.dev`) may start with a shallower proof-of-work requirement than a subdomain or a hosted
identity (`alice.users.origout.dev`). This is an **initial admission-cost policy**, full stop — it is not a trust
signal, a reputation boost, or a claim that root-domain identities are "stronger." Once a peer has an interaction
history with a given node, that node's own observations (§8) determine friction going forward, regardless of how the
identity was originally named.

## 5. Repository Discovery

### 5.1 Two identifiers, two layers

Origout distinguishes a **repository address** (the human-readable form) from a **RepoID** (the cryptographic,
network-level identifier):

```
Human address:    example.com/project
Network identity: RepoID = hash(repository public key)
```

A DNS repository record resolves an address to a RepoID, and may additionally list **preferred hosts** — routing hints,
not authorities. A repository is still verified cryptographically against its RepoID regardless of which host served it,
and if the preferred hosts are unreachable, discovery falls back to the wider network.

### 5.2 Clone flow

```
git clone og://example.com/project

1. Resolve the DNS repository record
2. Obtain the RepoID (and preferred hosts, if listed)
3. Contact an available peer
4. Verify the repository cryptographically against the RepoID
5. Fall back to DHT-based discovery if no preferred host answers
6. Retrieve Git data
7. Retrieve Origout collaboration metadata, where applicable
```

### 5.3 DHT fallback and what it actually protects

When preferred hosts aren't reachable, the client queries a DHT for the RepoID and verifies whatever candidate peers
return:

```
DNS -> RepoID -> DHT -> candidate peers -> cryptographic verification -> repository
```

Because `RepoID = hash(repository public key)`, a peer can't hand back forged repository content and have it verify —
this is an **integrity** guarantee. It is not an **availability** guarantee. An attacker can still eclipse a node,
suppress discovery responses, or delay data to disrupt whether a repository can be found at all, even though they can't
make it lie about what it is. Both properties matter, and conflating them overstates what the design actually protects.

## 6. Authority and Delegation

### 6.1 Capabilities

Authorization is expressed as namespaced capabilities:

```
repo.view
issue.create
issue.moderate
patch.create
patch.review
branch.write:main
delegate.issue
```

### 6.2 Delegation certificates

A repository owner (or someone the owner has delegated to) grants capabilities via a signed certificate:

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

Delegation chains root at the repository owner. An issuer can never grant a capability it doesn't itself hold.

### 6.3 Delegation depth

To keep a single compromised or over-eager key from flooding a repository with new grants, delegation is rate-limited by
distance from the owner:

| Ring | Who               | Grants per 10 days |
|------|-------------------|--------------------|
| 1    | Owner             | 32                 |
| 2    | Invited by owner  | 16 per inviter     |
| 3    | Invited by Ring 2 | 8 per inviter      |
| 4    | Invited by Ring 3 | 4 per inviter      |
| 5    | Invited by Ring 4 | 2 per inviter      |
| 6+   | Invited by Ring 5 | 0                  |

These limits, and the maximum depth (default 6), are configurable per repository and per capability type. It's worth
being explicit about what "Ring" means here: it describes **how far delegated authority has propagated from the owner**,
purely for authority containment and blast-radius reduction. It is not a trust tier, and a Ring 4 user is not "less
trusted" than a Ring 2 user — they simply sit further from the root of the delegation tree.

### 6.4 Repository policy and community delegation

A repository owner can define a signed, repository-wide policy for unauthorized or undelegated users, e.g.:

```
unauthenticated:
  - repo.view
  - issue.create
```

In practice, most new contributors don't get capabilities directly from the owner — they get them from a maintainer who
already has `delegate.*` capabilities, after the ordinary kind of community interaction (a Discord thread, a good first
PR, a forum post). That's not a protocol mechanism; it's just how communities already work, and delegation gives it a
cryptographic trail.

## 7. Revocation

### 7.1 Transitive revocation

A delegation is valid only while its issuer remains authorized. Revoking a key or a delegation automatically invalidates
everything that key delegated downstream.

```
Revocation {
  target (delegation_id or public_key)
  signature
}
```

Revocations are signed, propagate over the P2P network on a high-priority channel, and are enforced locally by every
node that receives them. A revocation is exclusively about **authority** — it does not touch any local peer-history
state (§8), and it never becomes a network-wide judgment about the revoked peer's behavior.

### 7.2 Revocation races

Because propagation isn't instantaneous, a just-revoked key could in principle still get a destructive action (a
force-push, a branch deletion) through before the revocation arrives. Origout addresses this with a short hold period on
destructive actions — e.g., 5 minutes — during which a late-arriving revocation cancels the pending action instead of
letting it apply. This is a consistency mechanism, not a trust mechanism.

### 7.3 Expiry

Delegations may carry an expiry timestamp to bound blast radius and force periodic review.

## 8. Anti-Abuse and Local Peer Familiarity

Origout doesn't ask "is this peer trustworthy?" — a question that has no good decentralized answer. It asks a narrower
one: **how much of my own resources am I willing to spend interacting with this peer?**

### 8.1 Proof of work

PoW is the fundamental admission mechanism. Difficulty can scale with a node's current load, the cost of the requested
operation, and — critically — the requesting peer's history with *that specific node*. It never scales with a globally
propagated score, because no such score exists.

### 8.2 Local peer history

Each node keeps private state about the peers it has actually interacted with:

```
Peer X
   |
   v
My node
   |
   +-- clean history      --> lower PoW
   +-- suspicious behavior --> higher PoW
```

This record is never gossiped, published, or treated as a protocol assertion. It isn't transferable between nodes, and
it isn't authoritative over anything — another node forms its own, independent view of the same peer.

- **Positive history** reduces PoW difficulty gradually, on purpose — one clean request shouldn't zero out friction
  instantly.
- **Negative history** increases friction progressively: a minor protocol failure raises friction slightly, repeated
  failures throttle harder, and a confirmed abuse pattern can reset a peer to maximum PoW or local rejection. Not every
  malformed message is treated as deliberate abuse.

### 8.3 What this doesn't solve

Origout doesn't hide the tradeoff it's making. A patient attacker can create an identity, behave well long enough to
earn low friction with a given node, and then abuse that standing. This is the accepted cost of choosing local, bounded,
understandable friction over a globally propagated reputation system — one that would bring its own consensus and
false-reporting problems. It's a deliberate design decision, not an oversight.

*(An earlier design considered network-propagated trust scores and a "Proof of Misbehaviour" mechanism for broadcasting
evidence of bad behavior. Both were removed: they made network-wide claims about a peer's conduct without any real
consensus mechanism behind them. Anti-abuse policy in Origout stays local to the node that observed it.)*

## 9. Repository and Collaboration Data

### 9.1 Code

Repository code is a standard Git repository. Canonical branches are maintained by authorized keys, and commits and ref
updates are signed.

### 9.2 Collaboration metadata

Issues, comments, reactions, reviews, projects, and moderation actions are **not** stored in Git. They're recorded as
signed events, replicated over the P2P network, and materialized locally into a SQLite database that lives inside the
repository directory (gitignored, but human-readable for inspection):

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

Events are append-only, independently verifiable offline, and prunable via compaction. This event log covers
Origout-specific actions only — Git tracks code history itself.

### 9.3 Conflict resolution

Because collaboration events can arrive at different nodes in different orders, Origout resolves conflicts with a
strict, owner-defined precedence list (e.g. `[Owner, Maintainer A, Maintainer B, …]`). State-changing actions initially
produce a temporary local state; a higher-precedence user can later broadcast a final state that overrides temporary
states from anyone ranked below them. Ties don't exist — the list is strict, and the owner's actions are always final.
This gives every node a deterministic way to converge without coordination, and it's derived entirely from delegated
authority, not from any behavioral score.

### 9.4 Retention

Owners define retention policy for the event log. If a repository's metadata grows past a configured size, nodes may
refuse new pushes until it's pruned or the limit is raised.

## 10. Networking and Cache Nodes

### 10.1 Node roles

Every user's Origout installation participates as a peer. **Cache nodes** are infrastructure peers that improve
availability and bootstrapping — anyone can run one by hosting the binary, with no special configuration or compensation
model required. Cache nodes have no authority over repository content or governance; see §15 for the boundary between
infrastructure moderation and repository governance.

### 10.2 Cache node policy

What a cache node stores is entirely up to its operator. Available policy levers include:

- repository size limits (e.g. a 1GB default)
- allowlists / denylists
- manual approval vs. automatic acceptance
- storage quotas
- custom validation scripts
- activity-based retention
- manual pinning to prevent eviction
- first-N acceptance rules

There's no network-wide, trust-weighted replication calculation. Cache nodes can still announce their inventory and
coordinate replication where that's useful, but the decision to store a given repository belongs entirely to the
operator: *"distributed infrastructure does not require distributed social consensus."*

Evictions are soft: when a cache node drops a repository from active storage, it enters a pending-deletion state (e.g. 7
days) rather than disappearing immediately, so it can still honestly answer replication challenges during the window.
Deleted repositories can always be re-pushed by their owner or another peer holding a copy.

### 10.3 Proof of Replication

A node can locally challenge a peer that claims to be storing a repository or LFS object, and determine whether that
peer can actually produce it. A failed challenge affects the *challenger's own* future interaction policy with that
peer — it doesn't create a network-wide reputation event or get broadcast anywhere.

### 10.4 Transport

Encrypted, multiplexed P2P transport (implementation currently: libp2p in Go), plus native Tor onion-service support for
nodes that want it. Nodes also answer plain `git pull` requests for code only, without Origout metadata, for
interoperability.

### 10.5 Bootstrap and node indexes

New nodes join the network via **node indexes**: lightweight, hostable JSON endpoints listing known peers (see §16).
They exist purely to bootstrap discovery — they are not authoritative registries, reputation authorities, or governance
of any kind, and a client can and should use several independent ones at once.

## 11. LFS

Origout supports Git LFS natively, distributed over the same P2P network as everything else.

- A user's local node runs an LFS server that ordinary Git LFS clients talk to.
- LFS objects are distributed to cache nodes and peers independently of Git object storage, with their own quotas.
- Files replicate whole by default; files over 10GB can optionally be chunked for partial retrieval and parallel
  download from multiple peers.
- Replication targets geographic spread (a minimum of a few nodes across different regions) rather than a raw
  percentage.
- Proof of Replication periodically verifies that a node still holds what it claims to.

Cache-node policy governs everything else: a node sets its own LFS size limits and can require higher PoW for LFS
operations given their cost, but none of this is trust-weighted. Where the earlier design said *"high-trust repositories
receive better LFS replication,"* the current model says:

> LFS replication and retention are controlled by cache-node policy, storage availability, repository activity, and any
> replication requirements the repository itself configures.

And where it said *"low-trust users require higher LFS PoW,"* the current model says:

> Nodes may require higher PoW for expensive LFS operations based on local peer history, operation cost, and current
> resource pressure.

## 12. Git Integration

### 12.1 Transparent CLI wrapper

Origout wraps Git transparently. It can be aliased so every `git` invocation goes through it; unrecognized commands pass
straight through to Git, and Origout adds its own on top:

```bash
git clone <repo-id>     # Origout handles P2P clone
git commit -m "fix"     # passed to git normally
git push                # Origout adds P2P hooks
git webui               # Origout-specific: launch local UI
git log                 # passed to git normally
```

### 12.2 Distribution model

The main binary (`origout`, aliased `og`) dispatches to specialized tools the way `git` dispatches to `git-<command>`:
`origout-node` for P2P server/node management, `origout-wui` for the local web UI, `origout-mail` for the local mail
server, `origout-node-index` for bootstrap index generation, and so on. Every command accepts a versioned `--json`
output flag, which is what the web UI and mail server are themselves built on top of — one source of business logic,
several interfaces.

### 12.3 Local web UI

`origout webui` serves a GitHub-like interface on localhost, using the machine's own keys — it isn't meant for public
hosting. It's functional without JavaScript and enhanced with it, aims for something between SourceHut's simplicity and
GitHub's friendliness, and ships dark/high-contrast theming as a first-class feature. (Implementation detail, not a
protocol requirement: currently Go + Fiber + Templ + GORM.)

### 12.4 Local mail server

For teams that prefer patch-based, email-driven workflows, `origout mail start` runs local SMTP/IMAP servers compatible
with ordinary mail clients and `git send-email`. Real email addresses never touch the network — only cryptographic
identities do. Issues, PRs, and comments show up as threaded mail locally, and outgoing mail is translated back into
normal Origout actions, so someone using email, someone using the web UI, and someone using the CLI can collaborate on
the same repository without noticing the difference.

### 12.5 Official mirrors and migration

Projects can maintain GitHub/GitLab mirrors that pull from a trusted Origout node, with the owner able to sign an
endorsement of the "official" one. Import tooling for existing GitHub/GitLab history, issues, and PRs is planned; since
that history predates Origout, it necessarily arrives unsigned, and how to represent that is left to be resolved in
implementation.

## 13. Web/WASM Client

Origout compiles to WebAssembly to provide a browser-based client that behaves as an ephemeral peer:

```
Browser -> Origout WASM -> DNS -> RepoID + hosts -> Origout peer
```

A static website can ship this client without being a centralized Origout backend itself — the site just distributes the
client; the client talks to arbitrary Origout peers. A node may also optionally serve the same static client locally,
but that hosted copy isn't authoritative in any way — it's just another client.

## 14. Anonymous Read-Only Browsing

An anonymous client — someone browsing without an Origout identity — can retrieve public repository data without a full
clone: metadata, HEAD, trees, the README, and specific blobs or commits on request.

```
repository page -> HEAD -> tree -> README -> user selects file -> fetch blob
```

This gives Origout a web-browsing experience without requiring a centralized forge backend. An anonymous client has no
persistent identity, no delegation, and no authority — it's read-only, faces higher PoW than an authenticated peer, and
cannot create issues or PRs, comment, react, review, modify branches, or perform moderation.

## 15. Moderation and Repository Policy

Origout separates two kinds of moderation that are easy to conflate:

```
Repository governance          Infrastructure moderation
        |                                |
owner + delegated authority       node operator policy
```

**Repository governance** — deciding what happens inside a repository's collaboration state (closing an issue, banning a
contributor from participating, moderating comments) — flows entirely from the owner's delegated authority (§6), and a
ban or block is just a normal capability revocation. It has no separate reputation side effect.

**Infrastructure moderation** — deciding what a given cache node stores or serves — is entirely the node operator's call
(§10.2), and is independent of repository governance.

Origout also carries an advisory reporting mechanism: users can flag a repository (adult content, copyright, illegal
content, etc.), and these reports are gossiped over the network purely as information. They have no protocol-level
effect — there's no voting, no quorum, and nothing automatically goes read-only. A cache-node operator may optionally
consume the report stream to inform their own local policy, or ignore it entirely. A repository stays reachable as long
as at least one node keeps serving it.

Cross-repository references in issues and metadata are informational only — the protocol doesn't verify that a
referenced repository or issue actually exists.

## 16. Bootstrap and Discovery Infrastructure

**Node indexes** (§10.5) are lightweight JSON endpoints hostable anywhere — static file hosting, a personal server, CI.
The binary ships with a diverse, hardcoded default list spanning official, community-run, and individual sources, so no
single index is a single point of failure, and indexes can self-update by periodically re-scanning the peers they know
about.

**Pre-1.0 version enforcement.** Before the protocol stabilizes, clients restrict themselves to the official bootstrap
node and enforce mandatory, signed updates, to avoid network fragmentation during active protocol development. This
restriction is removed at 1.0.

**CI/CD.** Origout doesn't mandate a CI provider — self-hosted runners, GitHub Actions, GitLab CI, or anything else can
hook in via webhooks and event triggers.

**Donations.** An `origout donate` command can surface donation metadata (Sponsors, Patreon, a wallet address, whatever
a maintainer lists) for frequently used nodes. It's informational only — no protocol-level payment handling.

## 17. Validation and Protocol Flows

### 17.1 Clone

```
User -> git clone og://example.com/project
     -> DNS resolution -> RepoID + preferred hosts
     -> connect to peer -> PoW challenge (per local policy) -> solve
     -> repository exchange -> verify RepoID / signatures
     -> Git objects -> local repository
```

### 17.2 Authenticated write

```
signed event/action
        |
        v
   PoW gate (admission / resource control)
        |
        v
 signature verification
        |
        v
 delegation resolution
        |
        v
  capability check
        |
        v
  revocation check
        |
        v
   expiry check
        |
        v
 repository policy
        |
        v
local anti-spam rules
        |
        v
     accept
```

The exact ordering can be optimized in implementation, but the conceptual split must hold: **PoW governs admission and
resource cost. Signatures and delegation govern authorization.** They're never merged into a single check.

## 18. Key Management

Origout uses full self-custody. Users are responsible for storing their own private keys — there's no recovery
mechanism, and a lost key means lost ownership. Backup strategy (encrypted backups, hardware keys, whatever fits) is
entirely on the user.

A user can rotate keys by publishing a signed rotation record and updating their DNS binding; delegations tied to the
old key are handled through the same revocation mechanism as any other authority change (§7). A repository owner can
similarly rotate the repository's operative key or transfer ownership outright, at their discretion.

## 19. Forking

Anyone can hard-fork a repository: pick a new root key, keep or discard prior history, and get a new RepoID. No protocol
coordination is required or possible — forking is unilateral. Your fork, your repository, and it's on you to get anyone
else to use it. That friction is intentional: it filters for real community splits instead of producing endless
abandoned forks.

## 20. Security Properties and Threat Model

### 20.1 What Origout actually claims

- Identity creation is inexpensive.
- Resource consumption by unfamiliar peers is bounded by PoW.
- Local peer history reduces friction for peers a given node already knows.
- Cache-node storage decisions are locally controlled.
- Cryptographic identity prevents forged repository authority.
- Delegation and revocation give explicit, auditable authority control.
- **Origout does not provide global Sybil immunity**, and doesn't claim to.

Put plainly: *Origout doesn't try to make Sybil identities impossible or globally distinguishable. It makes abusive
interaction cost computational resources, and keeps the consequences of observed behavior local to whichever node
observed it.*

### 20.2 Properties preserved end to end

- **Cryptographic authority** — every action is authenticated by a signature and an authorization chain.
- **Offline verification** — repository state and delegation structure can be checked independently, given sufficient
  data.
- **Explicit ownership** — every repository has a cryptographically identifiable owner.
- **Transitive revocation** — revoking a source invalidates everything delegated from it.
- **Local anti-abuse enforcement** — any node can independently impose PoW, throttling, or storage limits.
- **Censorship resistance** — no cache node or DNS record is the sole party able to change repository state. (This comes
  from independent peers being *able* to keep serving and redistributing data — not from a guarantee that any given
  repository stays available forever. A repository with zero remaining seeders is unavailable, full stop.)
- **Graceful degradation** — a single node choosing to impose more friction on a peer doesn't cause the rest of the
  network to reject that peer.

### 20.3 Threat model

| Threat                                                              | Defense                                                                                                                     | Limitation                                                                      |
|---------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| **Identity forgery** — impersonating a repository owner             | Public-key signatures, RepoID binding, delegation verification                                                              | —                                                                               |
| **DNS manipulation** — altering DNS records                         | DNS is a discovery layer only; cryptographic identity is independently verifiable; clients flag unexpected identity changes | Discovery convenience still depends on DNS availability                         |
| **DHT poisoning** — false repository locations                      | RepoID / content verification, multiple peers, fallback discovery                                                           | Availability can still be attacked even when integrity can't be forged          |
| **Sybil creation** — many cheap identities                          | PoW, rate limiting, local peer history, per-cache-node policy                                                               | Identities remain cheap; a patient attacker can grind a favorable local history |
| **Resource exhaustion** — CPU, bandwidth, storage, connection slots | PoW, rate limits, quotas, cache policy, local peer history                                                                  | —                                                                               |
| **Compromised delegated key**                                       | Scoped capabilities, delegation depth, expiry, revocation, precedence rules, destructive-action hold periods                | —                                                                               |

Note also what's explicitly out of scope: raw network-level DDoS (packet floods) requires infrastructure-level
protection, not a software-level answer.

## 21. Operational Tradeoffs and Residual Risks

Removing propagated trust doesn't remove risk — it relocates it. These tradeoffs are accepted deliberately, not
accidentally:

- **Patient Sybil.** A peer can build a clean local history with a node before abusing it. Origout doesn't claim
  otherwise.
- **Divergent local knowledge.** Node A's view of peer X can differ meaningfully from node B's — that's inherent to
  keeping history local rather than gossiped.
- **Cache fragmentation.** Without globally coordinated, trust-weighted replication, cache-node storage may end up less
  efficient than a centrally coordinated system. Accepted in exchange for local policy control and much lower protocol
  complexity.
- **DNS dependency.** DNS is external infrastructure and can go down or be manipulated. Cryptographic identity prevents
  that from becoming cryptographic authority, but discovery convenience still depends on DNS being reachable.
- **PoW cost.** New and unfamiliar users pay a real computational cost. That's the explicit price of not running a
  global reputation system.

## 22. Comparisons

### 22.1 Radicle

| Aspect          | Radicle             | Origout                   |
|-----------------|---------------------|---------------------------|
| Authority       | None                | Explicit owner            |
| Canonical state | No                  | Yes                       |
| Moderation      | Not really possible | First-class, owner-driven |
| Permissions     | Implicit            | Capability-based          |
| Revocation      | Weak                | Transitive                |
| Anti-abuse      | Basic               | PoW + local peer history  |
| Data storage    | Git-only            | Git + SQLite              |
| Infrastructure  | Seed nodes          | Cache nodes               |
| Git integration | Separate tooling    | Transparent wrapper       |
| Web UI          | Separate            | Local, built-in           |

### 22.2 Tangled

Tangled composes a decentralized forge out of multiple interoperating services and application layers. Origout takes a
narrower architectural bet: make the Git collaboration primitive *itself* directly addressable and distributable over
peers, rather than composing a forge experience on top. Neither approach is simply "more" decentralized than the other —
they scope the problem differently.

### 22.3 GitHub

| Aspect                | GitHub            | Origout                                  |
|-----------------------|-------------------|------------------------------------------|
| Infrastructure        | Centralized       | P2P + cache nodes                        |
| Authority             | Platform-mediated | Cryptographic                            |
| Data ownership        | GitHub, Inc.      | Repository owner                         |
| Access control        | Platform policy   | Owner-defined capabilities               |
| Anti-abuse            | Centralized       | PoW + local peer history                 |
| Discovery             | Central registry  | RepoID + DNS + cache nodes               |
| UI                    | Web-based         | Local web UI                             |
| Fork model            | Low friction      | Explicit, higher friction                |
| Censorship resistance | None              | Available while any peer serves the data |

The important UX point isn't in either table: **Origout is trying to feel closer to ordinary Git than to running a
decentralized social network.**

### 22.4 Fossil

| Aspect           | Fossil             | Origout                     |
|------------------|--------------------|-----------------------------|
| Metadata storage | SQLite in repo     | SQLite in repo (gitignored) |
| Infrastructure   | Self-hosted server | P2P network                 |
| Authority        | Repository admin   | Cryptographic owner         |
| Distribution     | Pull from server   | Pull from any peer          |
| Discovery        | Manual URL sharing | DHT + DNS                   |

Fossil's core insight — that collaboration metadata belongs in queryable storage, not Git commits — is one Origout
shares. The difference is distributing that metadata over P2P instead of requiring a central server.

## 23. Platform Scope

Origout is desktop-first: Linux, macOS, and Windows, assuming a full node's worth of disk, memory, and bandwidth.
There's no native mobile client planned; mobile access means remoting into a desktop or a personal always-on node (a
Raspberry Pi, for instance). That's a deliberate scoping decision to concentrate effort on the core P2P experience
rather than chasing cross-platform parity.

## 24. Conclusion

Origout gives Git a decentralized transport, discovery, collaboration, and authority layer, while keeping all of it
behind a workflow that still just looks like Git:

```
git clone og://example.com/project
git pull
git push
git log
```

Underneath that: DNS tells the protocol where to find an identity or a repository. Public keys determine identity.
Capabilities and delegation determine authority. Proof of work controls resource consumption from unfamiliar peers.
Local peer history controls how much friction any individual node chooses to apply. Cache nodes provide availability
without ever becoming authorities over what a repository *is*.

This isn't a decentralized trust machine. It's Git, with the infrastructure moved off of any one website's servers — and
the parts that make that possible left invisible unless you go looking for them.

## Appendix A: Optional Object-Storage-Backed Node Storage

*(A local implementation detail. Nothing in this appendix is visible to, negotiated with, or required by any other peer
on the network. It changes how a single node stores what it already has — not what it says, signs, or serves.)*

### A.1 Motivation

Section 10.2 leaves what a cache node stores, and how, entirely up to its operator. What it doesn't yet address is a
*storage engine* option for operators whose node has outgrown "just put the repos on disk."

A node running on a laptop or a small home server has no scaling problem — plain Git repositories on a local filesystem,
as described throughout this document, are the right and sufficient design. But a node acting as a popular public
mirror, a donation-funded cache, or infrastructure for a large project can accumulate enough repositories, enough churn,
or enough cold long-tail data that a single NVMe volume becomes the binding constraint — not on correctness, but on
operational convenience: disk sizing, backup strategy, and recovery from hardware failure.

Cursor's ["Git at any scale"](https://cursor.com/blog/git-at-any-scale) describes a storage engine, *Continuity*, built
for exactly this class of problem: how does one hosting system keep an unbounded number of Git repositories, of wildly
uneven size and activity, without every repository permanently occupying a fixed amount of expensive local disk? Their
answer — treat local disk as a rebuildable hot cache, and an object store as the durable source of truth — was designed
for a centralized platform coordinating many machines. Origout has no such coordination problem to solve, and no shared,
mutually-trusted object store spanning the network. But the underlying storage pattern doesn't depend on any of that
machinery. It's a good idea about *one machine's disk*, wearing a distributed-systems costume it doesn't need to keep
for Origout's purposes.

This appendix adapts the pattern to a single, independently-operated Origout node.

### A.2 What this is not

To be unambiguous, given how the source material frames this:

- This is **not** a replacement for local Git storage. A node's canonical repository copies remain ordinary Git
  repositories on disk, exactly as specified in the rest of this document.
- This is **not** a network-level mechanism. It has no RepoID, no DHT entry, no wire format, no effect on any other
  peer. Two nodes storing the same repository may make completely different, mutually invisible decisions here.
- This is **not** required infrastructure. A node with no object storage configured behaves exactly as specified
  elsewhere in this document, with zero loss of function.
- This is **not** a durability guarantee the protocol relies on. Repository availability across the network still comes
  from peer replication (§10, §20.2), not from any single node's storage backend. This is purely about *how affordable
  it is for one operator to keep serving a lot of data*.

### A.3 The pattern

For a node that opts in, storage for a given repository (or, more precisely, for a given repository's Git object data)
splits into two tiers:

```
      writes                          reads
        |                               |
        v                               v
  local NVMe repo  <---- rehydrate ---- object storage
  (hot, rebuildable)                    (durable, source of truth)
```

- **Object storage holds the durable copy.** Every push a node accepts is written to object storage before it's
  considered committed — the packfile as one object, the resulting reference update recorded only once that write is
  confirmed. This mirrors Continuity's separation of a push into its two natural components: bulk object data, and the
  small atomic reference transaction that makes it visible. The reference transaction should not be considered final,
  and should not be advertised to peers, until the corresponding object-storage write is durable.
- **Local NVMe holds a working copy.** All actual Git operations — the DAG walks, the packfile reads, everything this
  document's own citation of the source article explains is miserable to do over a network filesystem — happen against a
  completely ordinary local Git repository, for exactly the performance reasons that source describes.
- **Eviction becomes safe.** Because object storage already has a durable copy, a node can drop a cold or
  rarely-accessed repository from local disk without any data loss, and simply rehydrate it from object storage the next
  time a peer requests it. This is what actually solves the scaling problem: disk usage is bounded by *working set*, not
  by *everything the node has ever agreed to store*.
- **Compaction stays local.** A node repacks its own working copy the same way any Git server does, on its own schedule,
  with no coordination required — there is no second replica of this node's data to keep in sync, because there is no
  second replica of this node. The repacked result can optionally be re-uploaded to object storage to keep the durable
  copy compact as well, but there's no correctness requirement forcing this to happen promptly.

None of this requires a write-ahead log with the linearizability guarantees Continuity builds for its multi-replica,
multi-writer setting. A single node has no concurrent-primary problem to solve — object storage here is a durability
backend for one writer, not a consensus substrate for several. Take the pattern; leave the consensus machinery, it isn't
needed here.

### A.4 Protocol and vendor neutrality

Origout speaks the S3 API surface (`PUT`, `GET`, conditional requests) and nothing vendor-specific. This is a deliberate
floor, not a preference for Amazon specifically: it's the de facto common denominator that AWS S3, Cloudflare R2, MinIO,
Ceph RGW/Object Gateway, Backblaze B2, and a self-hosted Garage instance all implement compatibly enough to be
interchangeable behind one client.

Concretely:

- A node targets any S3-compatible endpoint via ordinary configuration (endpoint URL, credentials, bucket).
- Nothing in Origout privileges a specific provider, region, or pricing model. "S3-compatible" means the protocol, not
  the company.
- An operator can run this entirely on infrastructure they own (MinIO or Ceph on their own hardware) with no third party
  involved at all, if the goal is scaling one node's disk footprint rather than outsourcing durability to a cloud
  vendor.
- Because Origout already runs on both personal machines and dedicated servers (§23), this has to stay a config-time
  opt-in, never a default and never a dependency the rest of the software assumes exists. A node with no object storage
  configured takes the plain-disk path described in the main body of this document; nothing about clone, push,
  replication, or discovery changes shape depending on which one a given node chose.

### A.5 Summary

|                        | Plain local storage (default) | Object-storage-backed (optional)             |
|------------------------|-------------------------------|----------------------------------------------|
| Applies to             | Any node                      | Nodes that opt in                            |
| Visible to other peers | —                             | — (identical either way)                     |
| Local disk holds       | The repository                | A rebuildable working copy                   |
| Source of durability   | The local disk itself         | Object storage                               |
| Eviction               | Not applicable                | Safe; rehydrates on demand                   |
| Coordination required  | None                          | None (single writer, no replicas)            |
| Dependency introduced  | None                          | An S3-compatible endpoint, operator-supplied |

This is a node-local storage optimization, not a protocol feature. It exists because one operator's disk is a real,
mundane constraint that scales independently of anything discussed in §10 through §16 — and because a good answer to
that constraint already exists and didn't need to be reinvented.


---

TODO, Add to the whitepaper: Origout is daemonless. The way it works is that per each command you run, a background
process works for a short time (~5min), doing the necessary operations (syncing, etc.). This makes it so if you don't
use Origout today, you don't need to worry about it always running in background, which is quite nice. This is only for
end-users; servers are always on, of course. 