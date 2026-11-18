# Deleting from a store that keeps history

MemSt is built to keep things: messages are appended, facts are superseded
and not removed, and the versioned repository can go back to earlier states.
A user who asks for something to be forgotten wants the opposite. This note
sets out what deletion means in MemSt, what is implemented, and what is
designed and not yet built.

Status, November 2026: levels 1 and 2 are implemented for notes written
through MCP, and for sessions as far as the search index. Everything else
below is design.

## Where content lives

A piece of text a user gave MemSt can be in any of these.

| Place | What it holds | Written | Can one item be removed today? |
|---|---|---|---|
| `sessions/<id>/messages.bin`, `messages.idx` | Every message of a session | Appended | No: only the whole session |
| `sessions/<id>/memory/*.bin` | Memory items by tier | Rewritten on change | Yes: `SessionStore::remove_memory` |
| `sessions/<id>/operations.log` | Tool calls with their inputs and outputs | Appended | No: only with the session |
| `search_index/` | A copy of each indexed text, and its terms | Updated | Only a whole session |
| `graph.helix` | Entities and relationships extracted from messages, each citing its source message; embeddings | Updated; superseded facts are kept | Entities and relationships one by one; not "everything from this message" |
| Embeddings | Vectors on memory items and entities | With the item | With the item |
| Versioned repository (`memst-repo`): objects, write-ahead log, checkpoints | Content-addressed snapshots, and the log of writes that produced them | Appended; history is the point | No |
| `mcp-audit.jsonl` | Who wrote and deleted which note. No content | Appended | Not needed |
| Outside MemSt | Backups, a model provider's logs of extraction calls, the client's own transcript | | Not by MemSt |

## Four levels

**1. Hidden.** The item stops being returned. It is still on disk.
Superseding a fact does this: the old fact leaves queries and search and
stays in history. This is right for a fact that changed, and wrong for
"forget this": the text is still there to be read, restored or leaked.

**2. Removed from the current state.** The item is deleted from the files
that make up the store as it is now, and from every index derived from them.
A search finds nothing; reading the store's current files finds nothing.
Earlier snapshots still hold it.

**3. Removed from history.** No snapshot, log or checkpoint holds it.
Going back in time does not bring it back.

**4. Removed everywhere.** Backups, provider logs and client transcripts
too. MemSt can list these; it cannot reach them.

A request to be forgotten means level 3 within MemSt, and an honest account
of level 4.

## What is implemented

- **Notes written through MCP: level 2, and nothing else holds them.**
  `memory_forget` rewrites the tier file without the note. These notes are
  never put in the search index, the graph or the repository, and the audit
  log records the deletion without the content. A test checks that the text
  is no longer anywhere under the store directory
  (`a_forgotten_note_is_gone_from_the_store_and_the_log`). Where the
  operating system or the disk keeps old blocks is outside MemSt; see below.
- **Sessions: level 2 for messages, memories, operations and the search
  index.** `SessionStore::delete_session` removes the session's directory
  and its entries in the search index. Before November 2026 it left the
  index untouched, so a deleted session's text stayed searchable.
- **Graph facts: level 1.** Superseding, deprecating and merging keep the
  old record. Entities and relationships can be deleted one at a time.

## What is not, and the design for it

### A. Deleting one message

Messages are an append-only file with byte offsets in an index. Design:

1. Rewrite `messages.bin` without the message and rebuild `messages.idx`,
   under the store lock, writing to a temporary file and renaming it over the
   original so a crash leaves either the old file or the new one.
2. Leave a tombstone in the index: the message's id, when it was deleted,
   and nothing else. Anything that cites the id can then say "deleted"
   instead of "not found", and a sync from another copy does not bring the
   message back.
3. Remove the message from the search index. This needs a delete-by-document
   operation in both index backends; today they can only delete a session.

### B. Cascading to what was derived

A message that is deleted has usually been used. For each derived item the
question is whether it still stands without that message.

- **Graph relationships** cite one source message (`source_message_id`).
  Delete the relationships whose source is the deleted message. A
  relationship that was stated again in another message was "reinforced",
  not stored twice, so today that second source is not recorded: add a list
  of supporting messages to each relationship, and delete the relationship
  only when the list becomes empty.
- **Graph entities** record the sessions they were seen in, not the
  messages. Delete an entity when no relationship and no remaining message
  refers to it. This needs a per-message record of the entities it produced,
  written at ingestion.
- **Memory items** carry a free-text `source`. Give memories derived from
  messages a structured list of source messages, and apply the same rule.
- **Summaries and consolidated memories** mix many sources. They cannot be
  edited to remove one source's contribution; they have to be marked stale
  and rebuilt from what remains.
- **Embeddings** go with their item. An embedding of deleted text is deleted
  text in another form: it can be inverted approximately.

The general rule: every derived item names the items it was made from, and
deletion walks those links. Without that record, deletion is guesswork. This
is the largest piece of work here, and it is the same provenance the
security review asks for.

### C. Deleting a session's facts from the graph

`delete_session` does not touch `graph.helix`. Until B is built, the
workable rule is by session: delete relationships whose session is the
deleted one, then entities seen only in that session. It should be part of
`delete_session`; it is not yet, because the session store does not depend
on the graph backend. The command line can do it after deleting the session.

### D. History

The versioned repository is content-addressed: an object's name is the hash
of its content, and commits name the objects they contain. Removing content
changes names all the way up. Three ways, in order of preference:

1. **Encrypt per item, and delete the key.** Each stored item is encrypted
   with its own key, kept in a small key table outside the content-addressed
   store. History holds only ciphertext. Deleting the key makes every
   snapshot's copy unreadable at once, without rewriting anything, and
   object names and commit hashes stay valid. This is the only approach that
   keeps the audit value of history while honouring deletion. Costs:
   identical content no longer shares storage, the key table becomes the
   thing to protect and back up, and search and embeddings are still
   plaintext and need A and B.
2. **Rewrite history.** Produce a new chain of commits without the item, as
   `git filter-repo` does, and discard the old objects, the write-ahead log
   archives and the checkpoints. It works with the store as it is. Every
   commit after the first change gets a new name, so anything holding an old
   name (another copy, a branch elsewhere) is broken, and a copy that still
   has the old objects can push them back.
3. **Bound history.** Keep snapshots for a fixed time, then drop them.
   Deletion reaches level 3 when the last snapshot containing the item
   expires. It is simple and predictable, and gives up the long history that
   is MemSt's reason to exist.

Recommendation: 1 for new stores, with 3 as a setting for those who want a
limit, and 2 as the tool for a store created before encryption.

MCP notes are not committed to the repository today, so this does not yet
block `memory_forget`. It does block "delete this message" for any store
that uses the repository.

### E. What a deletion leaves behind on purpose

A record that a deletion happened: which id, when, on whose request. Not the
content, and not a hash of it: a hash of a short text can be reversed by
guessing. This is what `mcp-audit.jsonl` already does for notes.

### F. What MemSt cannot delete

- **Disk blocks.** Rewriting a file does not overwrite its old blocks, and
  on solid-state disks and copy-on-write file systems it cannot be made to.
  Encrypting the store's disk is the answer; MemSt should say so and not
  claim secure erasure.
- **Backups and other copies** of the store.
- **A model provider's records** of the extraction and embedding calls that
  carried the text. MemSt can report which providers a store was configured
  to send text to.
- **The client's own transcript** of the conversation.

A deletion command should end by listing these, so that "forgotten" is not
read as more than it is.

## Checking it

For each level the test is the same shape as the one for MCP notes: put a
distinctive string in, delete it, and search every byte under the store
directory for it, including after reopening the store and after going back
to an earlier version. A deletion feature is done when that test passes for
the level it claims.

## Order of work

1. Delete-by-document in both search backends (needed by A).
2. Graph clean-up in `delete_session` (C).
3. Deleting one message, with tombstones (A).
4. Provenance links from derived items to their sources, and the cascade (B).
5. Per-item encryption in the repository (D.1).
