# Static types: function signatures and local inference.
# Notessë = Int, Quetta = Str, Nanwa = Bool, Er = Unit, Técma = List.
maita add(a: Notessë, b: Notessë) -> Notessë {
    entul a + b;
}

# Local types are inferred, like Rust's `let`.
ea x: Notessë = 5;
ea y = add(x, 2);
quete(y);

# Empty lists need context or an annotation.
ea names: Técma<Quetta> = [];
napan(names, "Elen");
napan(names, "Anar");
quete(names);
quete(not(names));
