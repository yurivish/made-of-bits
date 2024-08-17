## Future work

- Implement the compressed bit vector as described in "Fast, Small, Simple Rank/Select on Bitmaps": https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf
  - See also: https://observablehq.com/d/5370347688e58b4d
- Implement a quad vector and quad wavelet matrix. Explore its use for two-dimensional range queries without the need for Morton masks.
  - Paper: Faster wavelet trees with quad vectors: https://www.kurpicz.org/assets/publications/qwm_preprint.pdf
  - Paper: Faster Wavelet Tree Queries: https://arxiv.org/abs/2302.09239
  - Code: https://github.com/rossanoventurini/qwt
