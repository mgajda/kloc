# assets

`deepseek-v4-tokenizer.json` — the tokenizer for `deepseek-ai/DeepSeek-V4-Flash`,
distributed by DeepSeek under the MIT License (see LICENSE in the
`deepseek-ai/DeepSeek-V4-Flash` repository). It is a byte-level BPE tokenizer
(128,000 mergeable tokens) using the DeepSeek V3/V4 pretokenization scheme
(digits / CJK / main regex sequence), embedded in the binary and used by the
`--tokens` LLM token count via the gigatoken tokenizer.
