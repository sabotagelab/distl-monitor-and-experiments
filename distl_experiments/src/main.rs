use csv::Reader;
use std::time::Instant;

use distl::{self, FormulaNode, Ivl};

fn op_merge(
    arr: Vec<distl::FormulaNode>,
    op: distl::FormulaSymbol,
) -> Option<Box<distl::FormulaNode>> {
    if arr.is_empty() {
        return None;
    } else if arr.len() == 1 {
        return Some(Box::new(arr[0].clone()));
    }
    let mut left_arr = arr;
    let right_arr = left_arr.split_off(left_arr.len() / 2);
    Some(Box::new(distl::FormulaNode {
        symb: op,
        left: op_merge(left_arr, op),
        right: op_merge(right_arr, op),
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let always = |ivl, subformula: FormulaNode| {
        assert!(distl::valid_distl_ivl(&ivl));
        assert!(distl::valid_distl_atom(subformula.clone()));
        let b = ivl.lb.get_val();
        let c_min_b = ivl.ub.get_val() - b;
        distl::FormulaNode {
            symb: distl::FormulaSymbol::Eventually(Ivl::new(b, b, true, true)),
            left: Some(Box::new(distl::FormulaNode {
                symb: distl::FormulaSymbol::Until(Ivl::new(c_min_b, c_min_b, true, true)),
                left: Some(Box::new(subformula)),
                right: Some(Box::new(distl::FormulaNode {
                    symb: distl::FormulaSymbol::True,
                    left: None,
                    right: None,
                })),
            })),
            right: None,
        }
    };

    let op_preds = |n, op| {
        let preds: Vec<_> = (1..=n)
            .map(|i| distl::FormulaNode {
                symb: distl::FormulaSymbol::Pred(distl::Predicate {
                    agent: i,
                    cmp: distl::Cmp::Gte,
                    val: 0.,
                }),
                left: None,
                right: None,
            })
            .collect();
        *op_merge(preds, op).unwrap()
    };

    /* p /\ q /\ ... */
    let _f1 = |n| op_preds(n, distl::FormulaSymbol::And);

    /* F_[1,2] G_[0,3] (p \/ q \/ ...) */
    let _f2 = |n| {
        let disj_preds = op_preds(n, distl::FormulaSymbol::Or);
        let f_ivl = Ivl::new(1., 2., true, true);
        let g_ivl = Ivl::new(0., 3., true, true);
        distl::FormulaNode {
            symb: distl::FormulaSymbol::Eventually(f_ivl),
            left: Some(Box::new(always(g_ivl, disj_preds))),
            right: None,
        }
    };

    /* p U_[0,3] (G_[0,6] q \/ r \/ ...) */
    let _f3 = |n| {
        let left = Some(Box::new(distl::FormulaNode {
            symb: distl::FormulaSymbol::Pred(distl::Predicate {
                agent: 1,
                cmp: distl::Cmp::Gte,
                val: 0.,
            }),
            left: None,
            right: None,
        }));
        let right = Some(Box::new(always(
            Ivl::new(0., 6., true, true),
            op_preds(n, distl::FormulaSymbol::Or),
        )));
        distl::FormulaNode {
            symb: distl::FormulaSymbol::Until(Ivl::new(0., 3., true, true)),
            left,
            right,
        }
    };

    const N: usize = 4;
    let mut dsig: [_; N] = std::array::from_fn(|_| Vec::new());

    let mut durations = Vec::new();

    for j in 0..3 {
        for i in 0..N {
            let data_path = format!("data/sig_{}_10_0_{}.csv", j, i + 1);
            let mut rdr = Reader::from_path(data_path)?;
            let mut data: Vec<(f64, f64)> = Vec::new();

            for result in rdr.deserialize() {
                let pair: (f64, f64) = result?;
                data.push(pair);
            }
            dsig[i] = data;
        }

        let formula = _f1(N);
        assert!(distl::valid_distl(formula.clone()));
        let start = Instant::now();
        let _results = distl::compute(&dsig, formula);
        let duration = start.elapsed();
        println!("{},{}", N, duration.as_secs_f64());
        durations.push(duration.as_secs_f64());
    }
    // println!("Results length: {}", results.len());
    // println!(
    //     "Average time taken: {}",
    //     durations.into_iter().reduce(|x, acc| x + acc).unwrap() / 10.
    // );

    Ok(())
}
