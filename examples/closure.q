# A closure captures x.
maita adder(x) {
    maita add(y) {
        entul x + y;
    }
    # Return the inner function.
    entul add;
}

ea add5 = adder(5);
quete(add5(3));
