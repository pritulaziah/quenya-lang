# Function types and higher-order functions.
maita apply(f: maita(Int) -> Int, x: Int) -> Int {
    entul f(x);
}

maita inc(x: Int) -> Int {
    entul x + 1;
}

quete(apply(inc, 4));
