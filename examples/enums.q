# Enums and pattern matching.
cilmë Shape {
    Circle { radius: Notessë },
    Rectangle { width: Notessë, height: Notessë },
}

maita area(s: Shape) -> Notessë {
    nam s {
        Shape::Circle { radius: r } => {
            entul 3 * r * r;
        }
        Shape::Rectangle { width: w, height: h } => {
            entul w * h;
        }
    }
}

essë c = Shape::Circle { radius: 1 };
essë r = Shape::Rectangle { width: 3, height: 4 };
quétë(area(c));
quétë(area(r));
