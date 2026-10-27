# Fibonacci numbers.

# A recursive function.
maita fib(n: Notessë) -> Notessë {
    qui n < 2 {
        entul n;
    }
    entul fib(n - 1) + fib(n - 2);
}

# Print the first ten numbers.
ea i = 0;
ya i < 10 {
    quete(fib(i));
    i = i + 1;
}
