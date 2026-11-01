# Mutability, moves, partial moves, and clone.
canta Elda {
    quetta: Quetta,
    nótië: Notessë,
}

essë vistë elda = Elda { quetta: "Elen", nótië: 1 };
elda.nótië = 2;

# A Quetta is not Copy. This moves the field out of elda.
essë quetta = elda.quetta;
quétë(elda.nótië);
quétë(quetta);
# quétë(elda); # error: elda is partially moved

# clone borrows the value and returns a new one.
essë quetta2 = clone(quetta);
quétë(quetta2);

# napan needs a mutable list. clone keeps the first list unchanged.
essë vistë tengwar = [1, 2];
essë vistë anna = clone(tengwar);
napan(anna, 3);
quétë(tengwar);
quétë(anna);

# ilyë moves the list. tengwar is not usable after this loop.
ilyë tengwa mí tengwar {
    quétë(tengwa);
}
