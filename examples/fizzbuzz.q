# FizzBuzz from 1 to 15.
essë nótië = 1;
yallumë nótië <= 15 {
    qui nótië % 15 == 0 {
        quétë("FizzBuzz");
    } mal qui nótië % 3 == 0 {
        quétë("Fizz");
    } mal qui nótië % 5 == 0 {
        quétë("Buzz");
    } mal {
        quétë(nótië);
    }
    nótië = nótië + 1;
}
