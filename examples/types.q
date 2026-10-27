# Static types: function signatures and local inference.
maita add(a: Int, b: Int) -> Int {
    entul a + b;
}

# Local types are inferred, like Rust's `let`.
ea x: Int = 5;
ea y = add(x, 2);
quete(y);

# Empty lists need context or an annotation.
ea names: List<Str> = [];
napan(names, "Elen");
napan(names, "Anar");
quete(names);
quete(not(names));
