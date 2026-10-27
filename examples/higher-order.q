# Function types and higher-order functions.
maita apply(f: maita(Notessë) -> Notessë, x: Notessë) -> Notessë {
    entul f(x);
}

maita inc(x: Notessë) -> Notessë {
    entul x + 1;
}

quete(apply(inc, 4));
