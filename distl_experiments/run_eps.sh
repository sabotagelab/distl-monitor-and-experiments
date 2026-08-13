#!/usr/bin/env fish

# --- Configuration ---
# --- Function: replace a specific line in a file ---
function replace_line
    set file $argv[1]
    set line_number $argv[2]
    set new_text $argv[3]

    set tmp (mktemp)

    awk -v n=$line_number -v s="$new_text" '
        NR == n { print s; next }
        { print }
    ' $file > $tmp

    mv $tmp $file
end

function run_eps
    set fmla $argv[1]
    set eps $argv[2]

    set MAIN_PATH "./src/main.rs"
    set DISTL_PATH "../distl/src/lib.rs"
    set FMLA_LINE_NUM 115
    set PRINT_LINE_NUM 120
    set EPS_LINE_NUM 9

    set FMLA_CONTENT "        let formula = _$fmla(N);"
    set CSV_OUTPUT "results/$fmla-eps-results.csv"

    replace_line $MAIN_PATH $FMLA_LINE_NUM $FMLA_CONTENT

    for i in (seq 0.02 0.02 $eps)
        set EPS_CONTENT "const EPS: f64 = $i;"
        set PRINT_CONTENT "        println!(\"$i,{}\", duration.as_secs_f64());"

        replace_line $DISTL_PATH $EPS_LINE_NUM $EPS_CONTENT
        replace_line $MAIN_PATH $PRINT_LINE_NUM $PRINT_CONTENT

        # --- Run a cargo command ---
        cargo run -q --release >> $CSV_OUTPUT
    end
    echo "Results written to $CSV_OUTPUT"
end

# run_eps "f2" 0.2
# run_eps "f1" 6.0
# run_eps "f3" 0.5
run_eps "f2" 0.04
run_eps "f3" 0.12
run_eps "f1" 1.1
