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

function reset
    set MAIN_PATH "./src/main.rs"
    set DISTL_PATH "../distl/src/lib.rs"
    set MAIN_LINE_NUM 97
    set FMLA_LINE_NUM 115
    set PRINT_LINE_NUM 120
    set DISTL_LINE_NUM 8
    set EPS_LINE_NUM 9
    set MAIN_CONTENT "    const N: usize = 4;"
    set FMLA_CONTENT "        let formula = _f1(N);"
    set PRINT_CONTENT "        println!(\"{},{}\", N, duration.as_secs_f64());"
    set DISTL_CONTENT "const N: usize = 4;"
    set EPS_CONTENT "const EPS: f64 = 0.05;"

    replace_line $MAIN_PATH $MAIN_LINE_NUM $MAIN_CONTENT
    replace_line $MAIN_PATH $FMLA_LINE_NUM $FMLA_CONTENT
    replace_line $MAIN_PATH $PRINT_LINE_NUM $PRINT_CONTENT
    replace_line $DISTL_PATH $DISTL_LINE_NUM $DISTL_CONTENT
    replace_line $DISTL_PATH $EPS_LINE_NUM $EPS_CONTENT
end

reset
