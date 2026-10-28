# Fibonacci numbers.

# A recursive function.
maita nótië(n: Notessë) -> Notessë {
    qui n < 2 {
        entul n;
    }
    entul nótië(n - 1) + nótië(n - 2);
}

# Print the first ten numbers.
essë quen = 0;
yallumë quen < 10 {
    quétë(nótië(quen));
    quen = quen + 1;
}
