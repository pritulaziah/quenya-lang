# Function types and higher-order functions.
maita apply(f: maita(Quanta) -> Quanta, x: Quanta) -> Quanta {
    entul f(x);
}

maita inc(x: Quanta) -> Quanta {
    entul x + 1;
}

quete(apply(inc, 4));
