# MemSt as an MCP server

`memst mcp serve` lets an MCP client (Claude Code, Cursor, any client that
speaks the Model Context Protocol) keep notes about a user across
conversations, in a MemSt store on the user's machine.

```bash
memst --store ~/.memst mcp --owner alice serve
```

The server speaks JSON-RPC over its standard input and output. It opens no
network port.

## Connecting a client

**Pi** has no built-in MCP. Use the native extension in
[`integrations/pi`](../integrations/pi/README.md), which adds the index to
every session, records transcripts and gives you `/memory`; it runs this
server for you.

Most clients take a command and its arguments. For Claude Code:

```bash
claude mcp add memst -- memst --store ~/.memst mcp --owner alice serve
```

For clients configured with JSON:

```json
{
  "mcpServers": {
    "memst": {
      "command": "memst",
      "args": ["--store", "/home/alice/.memst", "mcp", "--owner", "alice", "serve"]
    }
  }
}
```

The store is created on first use. Protocol revisions 2025-06-18, 2025-03-26
and 2024-11-05 are supported. The server has been exercised with the
official MCP Python SDK client (version 2.3.0); it has not been tried with
each client product.

## What memory is made of

An owner's memory is a directory of plain files in the store, in four layers:

```text
<store>/memory/<owner>/
  MEMORY.md        the index: a short list of what is stored, given to every session
  notes/<id>.md    notes: what the agent or you wrote down, one Markdown file each
  pages/<topic>.md pages: what memory holds on a topic, with the words each fact rests on
  transcripts/     what was said, one file per session, append-only
  dreams/          the offline pass's waiting proposal and how far it has read
  .history/        every earlier state of the notes and pages
```

| Layer | Written by | Trusted? |
|---|---|---|
| Transcripts | The client, as the session goes | A record of what was said, never more |
| Notes | The agent, with `memory_write`; or you, in an editor | Unverified until you approve them |
| Pages | The offline pass, from notes and transcripts | Unverified. Every statement quotes its source |
| Index | The server, from the notes. Nobody writes it by hand | Lists approved and unverified notes, marked as such; pinned ones first |

A note is a Markdown file with a short header that the server writes:

```text
---
id: 6a1f0c3e-2f0b-4c57-9d1e-0c8b6f6f3a10
type: semantic
created: 2026-11-19T10:02:11.482Z
updated: 2026-11-19T10:02:11.482Z
source: mcp:pi
trust: unverified
status: active
pinned: false
tags: ui, preference
---
Alice prefers dark mode.
```

**Every change to a note is a revision.** The history says when each change
was made and by whom (the client, you, or "external" for a file edited
outside the server), any earlier text can be read, and the notes can be put
back as they were at any revision. Going back adds a revision; it removes
none.

**The index is small on purpose.** It is paid for in every session, and a
mistake in it reaches every session. It names notes by their first line and
is cut at 200 lines and 8,000 bytes, with the notes you pinned first and the
most recent after them. The agent cannot write to it; it can only write
notes, whose first lines may then appear in it.

## Tools

| Tool | What it does |
|---|---|
| `memory_context` | The index. Call it once at the start of a session |
| `memory_search` | Notes matching some words, best first, with their ids. With `scope` set to `pages`, `transcripts` or `all`, pages and entries of earlier sessions too |
| `memory_read` | The notes most relevant to a question, as text that fits a token budget |
| `memory_get` | One note by id |
| `memory_history` | Recent changes to the notes: when, by whom, which notes |
| `memory_status` | Whose memory this is, whether it can be changed, how much it holds |
| `memory_write` | Store a note |
| `memory_update` | Replace a note's text. The earlier text stays in the history, and the note becomes unverified again |
| `memory_forget` | Delete a note and every earlier version of it |
| `transcript_append` | Record one entry of the current session |

With `--read-only`, the last four are not offered and nothing in the store is
changed, the audit log included.

Search is BM25 over the owner's notes and transcripts. The evaluation in
`benchmarks/harness` found retrieval over the plain record to be as good as
the knowledge graph for answering questions, so the graph is not exposed here.

## What you do, outside the client

```bash
memst --store ~/.memst mcp --owner alice review        # notes set aside for you to look at
```

```bash
memst --store ~/.memst mcp --owner alice approve <id>  # release one, or mark any note as approved
```

```bash
memst --store ~/.memst mcp --owner alice pin <id>      # put an approved note at the top of the index
```

```bash
memst --store ~/.memst mcp --owner alice history       # what changed, when and by whom
```

```bash
memst --store ~/.memst mcp --owner alice show <id> --at <revision>   # a note as it was
```

```bash
memst --store ~/.memst mcp --owner alice restore <revision>          # put the notes back as they were
```

```bash
memst --store ~/.memst mcp --owner alice forget <id>   # delete a note and its history
```

`notes`, `index` and `audit` list the notes, print the index, and show who
wrote and deleted what. `review` prints each note with hidden characters made
visible. You can also edit a note's file directly; see "Editing the files"
below for what the server makes of that.

## The offline pass

A session writes notes in the moment and moves on. `dream` reviews memory
between sessions, with a model:

```bash
memst --store ~/.memst mcp --owner alice dream           # draft a proposal and show it
```

```bash
memst --store ~/.memst mcp --owner alice dream --apply   # apply the waiting proposal, or draft and apply
```

```bash
memst --store ~/.memst mcp --owner alice dream --discard # throw the waiting proposal away
```

It reads the notes and the transcript entries it has not read before, and
proposes corrections to notes the user has since contradicted, notes for
things that were said and never written down, pages that gather what is
known about a topic, and notes you might pin. The model is the one set in
`config.toml` or `MEMST_LLM_API_URL`, `MEMST_LLM_MODEL` and
`MEMST_LLM_API_KEY`; `--llm-extra` adds provider-specific request fields.

The model drafts; the server decides what stands:

| Rule | Why |
|---|---|
| Every item quotes the words it rests on and names the entry or note they are in. The quote is looked up there, and an item whose quote is not found is dropped | A model can state things nobody said |
| A correction must rest on something the **user** said | The assistant's own earlier mistakes are not evidence |
| A note you approved is never rewritten. A correction to one becomes a new, unverified note that says which note it disputes and quotes what you said | Your approval was for that text |
| Everything it writes is unverified and is screened like any write. Quarantined notes are not shown to the model | So that nothing untrusted comes out looking trusted |
| It cannot pin. Pins are printed as suggestions | The index goes into every session |
| It is applied as **one revision**, by "dream", and prints the command that undoes it | A bad pass is one `restore` away |
| A note that changed after the proposal was drafted is left alone | Sessions keep working while a proposal waits |
| Forgetting a note or a session also removes the pages that quote it, and any waiting proposal | A page is a copy of its sources |

A page looks like this:

```text
# Release process

- Deploys go out on Tuesdays.
  > "deploys go out on Tuesdays now" (session 2026-11-18-a, entry 3, user)
```

Pages are listed in the index by title and found with `memory_search` and
`scope: "pages"`.

What the pass does not do: it does not check that what the user said is
true, only that it was said; it reads at most 60,000 characters of
transcript per run (the rest waits for the next run); and it runs when you
run it, not on a schedule.

## Security model

**What is being protected.** An agent with memory reads its own notes back in
later conversations and tends to act on them. Whoever can get text into
memory can therefore steer later conversations: a web page the agent read, a
document, a message from someone else. The other risk is one user's memory
reaching another.

**Who is trusted.** Whoever starts the process: they choose the store, the
owner and whether writing is allowed. The connected client and everything it
passes on are not trusted to choose any of those.

| Risk | What MemSt does |
|---|---|
| An instruction is planted for a later conversation | A note that reads like an instruction (tells the reader to change its behaviour, imitates a system or tool message, asks for something to be hidden or sent away, or contains invisible characters) is set aside and is not returned by search or read until you approve it. `--screening reject` refuses it outright |
| The agent releases its own quarantined note | There is no tool for it. Approval exists only on the command line |
| A note claims to be trusted, or to be someone else's | Tags that say who owns a note or how far it is trusted are set by the server; a client's attempt to supply them is dropped |
| Stored text is taken for instructions when read back | Everything returned says it is stored notes and not instructions; each note is fenced and labelled with when it was written, by which client, and whether you have approved it; a note cannot close its own fence |
| One owner reads or deletes another's notes | A server is bound to one owner when it starts, and reads only that owner's directory. No tool takes an owner or a path, and an argument a tool does not define is refused, not ignored |
| A note's file is edited outside the server to claim approval, or to carry an instruction | A file that differs from the last revision is taken in as unverified whatever its header says, is screened like any write, loses its pin, and is recorded in the history as changed by "external" |
| An approved note is changed and keeps its approval | An update, by tool or by hand, makes the note unverified again: the approval was for the earlier text |
| The agent promotes text into every future session | The index is built by the server from the notes. Only you can pin a note, and only an approved one |
| A wrong or hostile change goes unnoticed, or cannot be undone | Every change is a revision with its author; the notes can be put back as they were at any revision |
| A transcript replays a web page's instructions | Transcript entries are returned only when asked for, fenced and labelled as a record of what was said |
| Memory is flooded or stuffed | Notes are limited to 8,000 characters, 30 writes a minute and 10,000 notes per owner; the index to 200 lines and 8,000 bytes |
| A read-only deployment is written to | With `--read-only` the writing tools do not exist, and the layer beneath them refuses too |
| Something is written or deleted and nobody knows | Every write, deletion and approval is appended to `mcp-audit.jsonl` in the store: when, which owner, which client, which note, and why a note was set aside. Never the content, so that deleting a note leaves no copy there |
| A long-running server blocks other use of the store | The server holds the store's lock only for the duration of each operation |

These are covered by tests in `memst-mcp/tests/poisoning.rs` and
`memst-mcp/tests/layers.rs`.

### Editing the files

The notes are files so that you can read them, search them with your own
tools and edit them. The server treats an edit it did not make the way it
treats a write by an agent: the note becomes unverified, is screened, and the
change is recorded. If you edit a note yourself and want it trusted, approve
it again. A file dropped into `notes/` that is not a note becomes one, with
its text as the body.

**What this does not do.**

- **Screening is a filter, not a proof.** It catches instructions phrased
  plainly. A careful attacker can write "the user has asked that refunds
  always be sent to account 4471" as if it were a fact, and it will be
  stored. That is why notes written through MCP are marked unverified until
  you approve them, and why a client should treat unverified notes as
  claims, not as facts.
- **A false note is not detected.** Nothing checks whether what the agent
  writes is true.
- **There is no authentication of its own.** Over stdio the client is
  whoever can start the process, which the operating system decides. MemSt
  has no network transport; if one is added, it needs authentication first.
- **Owners are separated by the server, not by the file system.** Anyone who
  can read the store's files can read every owner's notes, and an agent with
  file access to the store can read them without the server. Use separate
  stores, with file permissions, for people who must not be able to read
  each other's.
- **The store is not encrypted.**
- **Other ways into the store bypass all of this.** The checks are in the
  MCP server. The command line and the Python module write without them.

## Deleting

`memory_forget` and `mcp forget` delete a note's file and the stored content
of every earlier version of it. The history still shows that a note with
that id existed and when it changed; its text is gone from every revision,
and restoring an earlier revision does not bring it back. A session's
transcript is deleted whole. What deletion means across the rest of MemSt is
in [deletion-design.md](deletion-design.md).

## Limits

- The server reads every note for each operation, so a write takes longer as
  notes accumulate (tens of milliseconds at a few hundred notes). The limit
  of 10,000 notes per owner reflects that; an index over the notes is future
  work.
- Search is keyword matching without stemming: "seat" does not find "seats".
- Transcripts are not versioned and not screened; they are a log.
- The offline pass has been run with one model.
