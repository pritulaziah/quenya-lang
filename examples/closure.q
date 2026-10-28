# A closure captures quetta.
maita anta(quetta: Notessë) -> maita(Notessë) -> Notessë {
    maita napan(quen: Notessë) -> Notessë {
        entul quetta + quen;
    }
    # Return the inner function.
    entul napan;
}

essë napan_5 = anta(5);
quétë(napan_5(3));
