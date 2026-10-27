# A closure captures x.
maita adder(x: Quanta) -> maita(Quanta) -> Quanta {
    maita add(y: Quanta) -> Quanta {
        entul x + y;
    }
    # Return the inner function.
    entul add;
}

ea add5 = adder(5);
quete(add5(3));
