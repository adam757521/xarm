# xarm-decoder

SIMD-accelerated decoder for fixed-length instruction words.

## ISAs Supported:
- [x] A32 (AArch32)
- [ ] T2 (Will be implemented if 2/4 byte classification is trivial)

## SIMD Extensions Supported:
- AVX512 (x86_64)

## Bugs:
- [ ] LDR hardcoded confusion

## Improvements:
- [ ] Scalar hotpath may use PEXT & branch instead of vectorized compare.
