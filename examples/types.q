# Static types: function signatures and local inference.
# Quanta = Int, Quetta = Str, Nanwa = Bool, Er = Unit, Técma = List.
maita add(a: Quanta, b: Quanta) -> Quanta {
    entul a + b;
}

# Local types are inferred, like Rust's `let`.
ea x: Quanta = 5;
ea y = add(x, 2);
quete(y);

# Empty lists need context or an annotation.
ea names: Técma<Quetta> = [];
napan(names, "Elen");
napan(names, "Anar");
quete(names);
quete(not(names));
