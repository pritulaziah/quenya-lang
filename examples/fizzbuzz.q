# FizzBuzz from 1 to 15.
ea i = 1;
ya i <= 15 {
    qui i % 15 == 0 {
        quete("FizzBuzz");
    } mal qui i % 3 == 0 {
        quete("Fizz");
    } mal qui i % 5 == 0 {
        quete("Buzz");
    } mal {
        quete(i);
    }
    i = i + 1;
}
