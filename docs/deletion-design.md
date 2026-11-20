# Deleting from a store that keeps history

MemSt is built to keep things: messages are appended, facts are superseded
and not removed, and the versioned repository can go back to earlier states.
A user who asks for something to be forgotten wants the opposite. This note
sets out what deletion means in MemSt, what is implemented, and what is
designed and not yet built.

Status, November 2026: notes written through the MCP server are removed at
level 3 (from the current state and from their own history). Sessions are
removed at level 2 as far as the search index. Everything else below is
design.

## Where content lives

A piece of text a user gave MemSt can be in any of these.

| Place | What it holds | Written | Can one item be removed today? |
|---|---|---|---|
| `sessions/<id>/messages.bin`, `messages.idx` | Every message of a session | Appended | No: only the whole session |
| `sessions/<id>/memory/*.bin` | Memory items by tier | Rewritten on change | Yes: `SessionStore::remove_memory` |
| `memory/<owner>/notes/*.md` and `.history/` | MCP notes, and every earlier version of each | Files, with a revision per change | Yes, with all its versions: `VersionedDir::purge` |
| `memory/<owner>/transcripts/*.jsonl` | What was said in each session | Appended | A whole session |
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

- **Notes written through MCP: level 3.** A note is a file with a revision
  per change. `memory_forget` deletes the file, records the removal as a
  revision, and deletes the stored content of every earlier version. The
  revisions themselves stay valid: they name each version by the hash of its
  content, and that name now leads nowhere, so the history shows that the
  note existed and reports its content as forgotten. This is the
  content-addressed form of design D below without the encryption: it works
  because a note's file carries a random id, so the hash left behind cannot
  be reversed by guessing the text. Restoring an earlier revision skips
  forgotten notes. Revision messages and the audit log carry ids, never
  text, and the index is rebuilt from the remaining notes. A test checks
  that the text is no longer anywhere under the store directory after a note
  with several versions is forgotten
  (`forgetting_removes_every_earlier_version_too`). Where the operating
  system or the disk keeps old blocks is outside MemSt; see below.
- **Transcripts written through MCP: level 2**, a session at a time, or
  entry by entry by the words an entry contains (`memory_forget` with
  `phrases`). They are not versioned, so there is no history to reach.
  Matching is by words, so a paraphrase survives.
- **Pages built from notes and transcripts: deleted with their sources.**
  A page records which notes and transcript entries its statements quote.
  Forgetting a note or a session purges every page that cites it, with the
  page's history, and discards any waiting proposal of the offline pass.
  This is the cascade of design B below, for the one kind of derived item
  MCP memory has. The pass rebuilds the pages from what remains.
- **Sessions: level 2 for messages, memories, operations, the search index
  and the graph.** `SessionStore::delete_session` removes the session's
  directory and its entries in the search index, and `memst session delete`
  first removes from the knowledge graph the relationships recorded by the
  session (every version) and the entities first seen in it that no other
  session saw and nothing else refers to (`KnowledgeGraph::forget_session`).
- **Single messages: level 2** (sprint 14). `SessionStore::delete_message`
  rewrites the message file and its index without the message, records the
  id and time in `deleted.idx` with nothing of the content, and removes the
  message's entry from the search index. `memst message delete` then removes
  from the graph the relationships extracted from that message and the
  entities first seen in it that nothing else refers to
  (`KnowledgeGraph::forget_message`). Tests search every byte of the store.
- **The search index really forgets.** With the tantivy backend a deleted
  document was only marked: its stored text stayed in its segment file until
  that segment happened to be merged. This was true of session deletion
  since it was fixed in sprint 6. Deleting now merges the segments and
  removes the unreferenced files, at the cost of a pass over the index; a
  test fails without it.
- **Graph facts otherwise: level 1.** Superseding, deprecating and merging
  keep the old record. Entities and relationships can be deleted one at a
  time.

Memory items that say which messages they were made from
(`MemoryItem::derived`) are removed with the message, and so are summaries
made from those (sprint 19); consolidation records its inputs this way.

What a deleted message does not reach: memory items that do not say where
they came from, the operations log, and a
fact that a later message restated (the graph records one source message
per fact, so the fact goes with the first; extracting again from what
remains brings it back).

## What is not, and the design for it

### A. Deleting one message *(built in sprint 14, as designed)*

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

### B. Cascading to what was derived *(graph relationships and entities by their one source message: built; the rest is not)*

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

### C. Deleting a session's facts from the graph *(built in sprint 14, in the command line)*

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

MCP notes use the simpler form described under "What is implemented":
delete the content, keep the name. That is enough for a single-copy store.
Per-item keys matter once stores are copied or synchronised, where a deleted
object can come back from another copy; a deleted key cannot be undone by a
copy that only has ciphertext.

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
