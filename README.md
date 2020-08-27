# Super Simple SAT (S3) Solver

A super simple SAT solver implementation.

- Implemented in 100% safe Rust!
- Runs on stable Rust channel.

## Credits

Heavily inspired by

- [PyCSCL's trivial SAT solver][trivial-sat-solver] and [JamSAT solver][jamsat-solver]
  by Felix Kutzner
- [Candy solver][candy-solver] by Markus Iser (udopia)

Thanks to their authors for the inspiration!

Also thanks to Holger H. Hoos for providing many useful `.cnf`
problems for benchmarks and tests [here][holger-h-hoos-benchmarks].

## Usage

### As Executable

```rust
cargo run --release <.cnf file>
```

This will print either `SAT` and the satisfying assignment it has found or `UNSAT`
depending on the input `.cnf` file.
You can find several random 3-SAT `.cnf` files in this repository's `cnf` directory
for testing and benchmarking.

#### Example: SAT

```
> cargo run --release ./cnf/sat/uf150-645/sat/uf150-001.cnf
start solving ...
SAT
model = 1 2 -3 4 -5 6 -7 8 -9 -10 11 -12 -13 14 -15 -16 17 -18 -19 20 -21 -22 -23 24 -25 26 27 -28 29 -30 31 32 33 34 -35 36 37 -38 39 40 41 -42 -43 -44 45 -46 47 48 -49 -50 -51 52 -53 -54 -55 -56 57 58 59 60 61 -62 63 -64 -65 66 67 68 69 70 71 -72 -73 -74 -75 76 -77 -78 79 80 -81 82 83 84 85 -86 -87 88 -89 90 -91 92 93 94 95 -96 -97 98 99 -100 -101 102 103 -104 105 106 -107 -108 109 110 111 -112 -113 -114 -115 -116 -117 118 119 120 121 122 123 -124 -125 126 127 128 129 130 131 132 133 -134 135 136 137 138 139 -140 141 142 143 144 145 -146 147 148 149 -150
```

#### Example: UNSAT

```
> cargo run --release ./cnf/uf150-645/unsat/uuf150-001.cnf
start solving ...
UNSAT
```

### As Library

How to use the S3Sat solver from a given `.cnf` input:

```rust
let cnf_input = br"
    p cnf 10 4
    1 3 5 0
    -2 -8 6 0
    -4 -8 -1 0
    -10 -7 -2 0
";
let mut solver = Solver::from_cnf(&mut &cnf_input[..]).unwrap();
let result = solver.solve(vec![]);
assert!(result);
```

How to use the S3Sat solver as a library:

```rust
fn clause(lits: &[Literal]) -> Clause {
    Clause::new(lits.into_iter().copied()).unwrap()
}

let mut solver = Solver::default();
let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]]));
solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]]));
solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]]));
solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]]));
let result = solver.solve(vec![]);
assert!(result);
```

## Development

### Run Tests

```
cargo test --release
```

### Run Benchmarks

```
cargo bench
```

## License

Licensed under either of

 * Apache license, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Dual licence: [![badge][license-mit-badge]](LICENSE-MIT) [![badge][license-apache-badge]](LICENSE-APACHE)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

[license-mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[license-apache-badge]: https://img.shields.io/badge/license-APACHE-orange.svg

[trivial-sat-solver]: https://github.com/fkutzner/PyCSCL/blob/master/cscl_tests/testutils/trivial_sat_solver.py
[jamsat-solver]: https://github.com/fkutzner/jamsat
[candy-solver]: https://github.com/Udopia/candy-kingdom
[holger-h-hoos-benchmarks]: https://www.cs.ubc.ca/~hoos/SATLIB/benchm.html
