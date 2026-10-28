# Structs, fields, and methods.
canta Point {
    x: Notessë,
    y: Notessë,
}

carasta Point {
    maita move(immo, dx: Notessë, dy: Notessë) -> Er {
        immo.x = immo.x + dx;
        immo.y = immo.y + dy;
    }

    maita sum(immo) -> Notessë {
        entul immo.x + immo.y;
    }
}

ea p = Point { x: 1, y: 2 };
p.move(10, 20);
quete(p);
quete(p.sum());
