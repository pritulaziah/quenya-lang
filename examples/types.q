# Static types: function signatures and local inference.
# Notessë = Int, Quetta = Str, Nanwa = Bool, Er = Unit, Técma = List.
maita anta(a: Notessë, b: Notessë) -> Notessë {
    entul a + b;
}

# Local types are inferred, like Rust's `let`.
essë nótië: Notessë = 5;
essë quanta = anta(nótië, 2);
quétë(quanta);

# Empty lists need context or an annotation.
essë essi: Técma<Quetta> = [];
napan(essi, "Elen");
napan(essi, "Anar");
quétë(essi);
quétë(notië(essi));
