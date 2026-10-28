# Lists, indexing, and iteration.
essë tengwar = [1, 2, 3];

# Add an element and change the first one.
napan(tengwar, 4);
tengwar[0] = 10;

# Print the list and its length.
quétë(tengwar);
quétë(notië(tengwar));

# Skip 2 and stop at 4.
ilyë tengwa mí tengwar {
    qui tengwa == 2 {
        lelya;
    }
    qui tengwa == 4 {
        rac;
    }
    quétë(tengwa);
}
