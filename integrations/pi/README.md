# MemSt memory for Pi

Gives the [Pi coding agent](https://pi.dev) a memory that outlives the
session. Pi has no memory of its own between sessions and no built-in MCP;
this is a native Pi extension that talks to the `memst` program directly.

What a session gets:

- **An index in the system prompt.** A short list of the notes in memory, by
  first line, with whether you approved each one. It is fetched once per
  session, so the prompt stays the same from turn to turn.
- **Tools for the model**: `memory_search`, `memory_read`, `memory_get`,
  `memory_history`, `memory_write`, `memory_update`, `memory_forget`.
- **A record of the session.** What you and the assistant say is appended to
  a transcript that later sessions can search. Tool output is not recorded.
- **`/memory` for you**: review and approve notes, pin, forget, see the
  history and go back, without leaving Pi.

The notes are Markdown files with a full history, kept in a MemSt store on
your machine. Nothing is sent anywhere by this extension. How memory is laid
out, and what protects it, is in [docs/mcp.md](../../docs/mcp.md).

## Install

You need `memst` on your `PATH` (build it from this repository with
`cargo build --release -p memst-cli`; the program is `target/release/memst`).

```bash
pi install /path/to/memst/integrations/pi
```

Or try it for one session without installing:

```bash
pi -e /path/to/memst/integrations/pi/extensions/memst.ts
```

## Settings

Environment variables, all optional:

| Variable | Meaning | Default |
|---|---|---|
| `MEMST_BIN` | The memst program | `memst` |
| `MEMST_STORE` | The store directory | `~/.memst` |
| `MEMST_OWNER` | Whose memory this is | your user name |
| `MEMST_READ_ONLY` | `1`: read memory, never change it, record nothing | off |
| `MEMST_SCREENING` | What happens to a note that reads like an instruction: `quarantine`, `reject` or `off` | `quarantine` |
| `MEMST_RECORD` | `0`: keep no transcript | on |

Use a different `MEMST_OWNER` or `MEMST_STORE` per project if you do not want
one project's notes in another's sessions.

## Using it

Nothing to do: tell Pi things, and ask about them later. To look at what it
has written:

```text
/memory                 the index, as the model sees it
/memory notes           every note, with its id and whether you approved it
/memory review          notes set aside because they read like instructions
/memory approve <id>    mark a note as checked by you
/memory pin <id>        put an approved note at the top of the index
/memory forget <id>     delete a note and every earlier version of it
/memory history         what changed, when, and who changed it
/memory restore <rev>   put the notes back as they were at a revision
```

Everything Pi writes starts as **unverified**, and the model is told so.
Approve the notes you have checked; pin the few that every session should
see first.

## What to know

- **Notes written by the model are claims, not facts.** Screening sets aside
  text that reads like an instruction, but a false statement written as a
  fact is stored. The index and every search result mark unverified notes.
- **Pi's file tools are kept out of the memory files**, so that notes are
  read and changed only through tools that record who did what. A shell
  command is another matter: `bash` is blocked only when the command names
  the memory directory outright, and a determined or confused model can
  reach the files another way. If it does, the next session takes any
  changed file in as unverified and screens it; it cannot grant itself
  approval. Reading another owner's notes that way is not prevented: use
  separate stores, with file permissions, for people who must not read each
  other's memory.
- **If `memst` is not installed**, Pi works as usual without memory and says
  so once.
- The transcript holds what was typed and answered, in plain text, in the
  store. Set `MEMST_RECORD=0` to keep none, or delete a session's file from
  `<store>/memory/<owner>/transcripts/`.

## Tests

```bash
cd integrations/pi && npm install && MEMST_BIN=../../target/debug/memst npm test
```

drives the extension with a stand-in for Pi against the real `memst`
program: no model is needed. With Pi installed and a model configured,

```bash
MEMST_BIN=target/debug/memst integrations/pi/test/live-recall.sh --model <provider/model>
```

runs two real sessions and checks that the second recalls what the first was
told. This was run with Pi 0.73.1 and passes; it needs a model and so is not
part of CI.
