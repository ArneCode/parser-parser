fn fib(n) {
    let i = 0;
    let a = 0;
    let b = 1;

    while i < n {
        print(num_to_str(a));

        let tmp = a + b;
        a = b;
        b = tmp;
        i = i + 1;
    }

    return 0;
}

fn main() {
    let n = str_to_num(input("How many Fibonacci numbers? "));
    return fib(n);
}

