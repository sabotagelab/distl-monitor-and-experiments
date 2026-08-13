fn main() {
    println!("Hello, world!");
    let formula: distl::FormulaNode = distl::FormulaNode {
        symb: distl::FormulaSymbol::Pred(distl::Predicate {
            agent: 1,
            cmp: distl::Cmp::Gte,
            val: 2.,
        }),
        left: None,
        right: None,
    };
    assert!(distl::valid_distl(formula));
}
