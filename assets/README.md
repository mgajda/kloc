# assets

Tokenizer specifications embedded in the binary, both used by the `tokens`
feature's LLM token counts via the gigatoken tokenizer. Both are byte-level
BPE; neither is redistributed from a provider feed — each is the model
author's own token spec.

- `deepseek-v4-tokenizer.json` — the tokenizer for `deepseek-ai/DeepSeek-V4-Flash`
  (byte-level BPE, 128,000 mergeable tokens, DeepSeek V3/V4 pretokenization:
  digits / CJK / main regex sequence), MIT-licensed by DeepSeek. Loaded via
  gigatoken's `load_hf_slice`.

- `claude.json` — the official tokenizer from
  `anthropics/anthropic-tokenizer-typescript` (the same file ships in
  `@anthropic-ai/tokenizer`). Its `bpe_ranks` is tiktoken's compressed format
  (`! <offset> <base64 tokens>`); the GPT-2 regex is the `pat_str`. MIT-licensed
  by Anthropic. Loaded via gigatoken's `from_ranks` + GPT-2 pretokenizer, and
  Anthropic's reference `countTokens` NFKC-normalizes before encoding, which we
  mirror. Verified token-for-token against `@anthropic-ai/tokenizer`.
