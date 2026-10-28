# Structs, fields, and methods.
canta Elda {
    quetta: Quetta,
    nótië: Notessë,
}

carasta Elda {
    maita vista(immo, quetta: Quetta) -> Er {
        immo.quetta = quetta;
    }

    maita notië(immo) -> Notessë {
        entul immo.nótië;
    }
}

essë elda = Elda { quetta: "Elen", nótië: 1 };
elda.vista("Anar");
quétë(elda);
quétë(elda.notië());
