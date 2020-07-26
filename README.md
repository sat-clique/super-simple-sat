# Super Simple SAT (S3) Solver

A super simple SAT solver implementation.

## Credits

Heavily inspired by [PyCSCL's trivial SAT solver][trivial-sat-solver].

## Usage

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

### Through library



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
