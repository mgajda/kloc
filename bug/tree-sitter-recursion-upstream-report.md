# tree-sitter 0.26.x: recursive C functions whose depth is driven by user data

**Status**: FILED — issue tree-sitter/tree-sitter#5807, fix PR tree-sitter/tree-sitter#5809.
**Drafted**: 2026-08-01 (local mitigation; filed upstream 2026-08-02).
**Source**: kloc profiling/handoff context — `tmp/HANDOFF.md` (session of 2026-08-01,
which found a stack-overflow abort in kloc's own walks and the C-recursive
`ts_node_child_with_descendant` underneath `ts_node_parent`).

## TL;DR

kloc (a tree-sitter-based metric tool) needs a hard guarantee that **no
unbounded recursion in the tree-sitter C library depends on external input**
(file size, tree depth). The parser core is already heap-stacked and
iterative, and most node/cursor accessors are iterative, but five internal
C functions still recurse with depth equal to the parse-tree depth — i.e. depth
controlled by the input document. A pathological input can overflow the C
stack through any of them, and (unlike `panic`) a C stack overflow in a
library call is not catchable by the host process.

The requested outcome: **make all input-driven recursion in `lib/src` iterative
or depth-bounded**, matching the in-tree precedent already used by
`ts_subtree_release` and `ts_node__child` (both rewritten iteratively with an
explicit stack).

## Audit basis

- tree-sitter tag **v0.26.11**, files `lib/src/{node.c, subtree.c,
  tree_cursor.c, parser.c, stack.c, tree.c, get_changed_ranges.c, lexer.c}`
  and `lib/include/tree_sitter/api.h`.
- Line numbers below are for v0.26.11.

## Recursive functions found (depth == tree depth, i.e. user-controlled)

| Function | File:line (self-call) | Reachable from | kloc hit? |
|---|---|---|---|
| `ts_node_child_with_descendant` | `node.c:560` (recurse at `:581`) | `ts_node_parent` (`:552`), `ts_node__prev_sibling` (`:195`), `ts_node__next_sibling` (`:251`) | no (kloc no longer calls `parent()`; walks use cursors) |
| `ts_subtree_has_trailing_empty_descendant` | `node.c:176` (recurse at `:183`) | `ts_node__prev_sibling` (`:215`), `ts_node__next_sibling` | no |
| `ts_node_child_by_field_id` | `node.c:601` (tail-call `goto recur` `:638`, non-tail recurse `:645`) | field-based lookups (`ts_node_child_by_field_name`, `ts_node_field_name_for_child`) | no |
| `ts_subtree__write_to_string` | `subtree.c:821` (recurse through children ~`:868`) | `ts_node_to_sctring`, debug logging | no |
| `ts_subtree__print_dot_graph` | `subtree.c:1008` | `ts_tree_print_dot_graph` (debug) | no |

Depth driver in every case: the number of nested ancestors/descendants in the
user-supplied document. A 10^6-deep nesting would try to recurse ~10^6 frames
on the C stack (default 8 MiB main thread; comfortably overflowed).

## Functions verified NON-recursive (iterative / O(1)), for the record

- Parser core: LR loop `parser.c:2127`, `ts_parser__advance`, `ts_parser__balance_subtree`
  (`:1873`, explicit `tree_stack`), `ts_parser__select_tree` (`:842`; compare is
  iterative via `ts_subtree_compare` `subtree.c:596`), `ts_parser__check_progress` (`:1542`).
- Stack: `stack.c` — `stack_node_retain`/`stack__subtree_node_count`/`stack__subtree_is_equivalent`
  are O(1) metadata; stack is an explicit heap structure.
- Node access used by traversal: `ts_node_child` → `ts_node__child` (`node.c:139`, iterative
  descent loop), `ts_node_child_count`, `kind`/`is_named`/`kind_id`/`start_byte`/`end_byte`,
  `ts_node_utf8_text` (slice only).
- Cursor walks: `ts_tree_cursor_goto_first_child` (`tree_cursor.c:208`), `goto_next_sibling`
  (`:351`) — iterative.
- **Tree destruction**: `ts_subtree_release` (`subtree.c:565`) — iterative explicit
  `tree_stack`; a 64k-deep tree frees without recursion. This is the function called
  on every `ts_tree_delete`, i.e. every dropped `TSTree`.
- `ts_subtree_get_changed_ranges` (`get_changed_ranges.c:413`) — iterator-based.

## Proposed fix

For each of the five functions, rewrite iteratively with an explicit stack /
explicit work list, exactly as `ts_subtree_release` and `ts_node__child`
already do in-tree. Where full iteration is awkward (e.g. `ts_subtree__write_to_string`'s
field-name nesting), an explicit-stack pre-order with a `(node, field_name, alias)`
frame is sufficient.

Secondary hardening (does not fix the above, but bounds damage):
`TSParseOptions.progress_callback` (`api.h:102`) can abort a parse on
byte-offset or wall-clock budget — a soft *size/time* guard, not a
recursion/memory limiter (see kloc's analysis in `tmp/HANDOFF.md` §"Pending
decision"). Not a substitute for iteration.

## Out of scope here

- **Grammar external scanners**: each grammar crate's C `scanner.c` runs inside
  `ts_lexer__advance`; the progress callback does not fire inside a scanner and
  the library cannot bound its recursion. A scan-time guard in the scanner ABI
  (`TSLexer` gains a depth/lookahead budget) would be a separate upstream
  proposal.
- Recursion in the Rust `tree-sitter` binding itself: none found (`lib.rs` parse/
  `utf8_text` are non-recursive).

## Local mitigation already landed (kloc side)

- kloc's tree walks are iterative (`collect_comment_ranges`, `walk_tree` in
  `counter.rs` / `complexity.rs`); it never calls `node.parent()`/prev/next
  sibling, so none of the five functions are reachable from kloc today.
- Committed in kloc `2fb3b57`.

## Cross-links

- Post-mortem / profiling context: `tmp/HANDOFF.md` (2026-08-01 session;
  documents the 16k-deep stack-overflow abort and its fix).
- This draft: `bug/tree-sitter-recursion-upstream-report.md`.
