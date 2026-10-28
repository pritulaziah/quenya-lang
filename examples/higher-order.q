# Function types and higher-order functions.
maita carë(carmë: maita(Notessë) -> Notessë, quetta: Notessë) -> Notessë {
    entul carmë(quetta);
}

maita anta(quetta: Notessë) -> Notessë {
    entul quetta + 1;
}

quétë(carë(anta, 4));
