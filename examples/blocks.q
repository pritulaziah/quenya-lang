# Block expressions and match expressions.
maita describe(n: Notessë) -> Quetta {
    entul nam n {
        0 => "zero",
        1 => "one",
        _ => "many",
    };
}

essë x = { essë y = 2; y + 3 };
quétë(x);
quétë(describe(1));
quétë({ nam 1 { 1 => "one", _ => "other" } });
