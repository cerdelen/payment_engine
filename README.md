# Payments engine
Assumptions made
- As floating point numbers are not totally accurate we cannot use f64 or f32 for the amount fields. Instead I will be using a scaled i128 which should be enough as:
    - maximum positive value is (2^127 - 1) / 10 ^ 4
    - maximum negative value is - (2^127) / 10 ^ 4
This should be more than enough to handle any reasonable transfer amounts
In case an acounts combined held + available amt would overflow we print the i128 Max value as a fallback.
As long as transactions dont overflow held or available themselves they are valid even if the combined values would overflow.


Amt values are fractional with a precision of up to 4 after the decimal point. Allowed formats are "x", "x.x", "x.xx", "x.xxx", "x.xxxx".
The whole number string will be accepted up to the aformentioned over/underflow boundries

Output normalization will always include a decimal point

A dispute can only refer to a deposit

Decision regarding disputed deposit with insufficient account balance to unwind the deposit
For a payment processing agent it is sensible to not allow such a disputr as otherwise the it would be the one covering the difference

An account can never go into negative Balance

thought about scaling
the in memory store for past transaction grow unbounded with every new deposit.
Since only deposits are dipsutable (and resolve/chargeback) no other transactions need to be stored.
It could be argued that chargebacked deposits could be deleted as they cannot be disputed anymore.
In a more elaborate system one might look at other complexity improvements like splitting the stored transactions differently.
There are various ideas to be had.
One idea is to group them by timestamp and either write them to disk at some point or delete them meaning after x amt of time any transaction is indisputable.
Another idea is to store only the last x amount of transaction of any given user. This would mean only the past x amt of transaction of any user are disputable.

AI Usage
- writing serde serializer for decimal points with a precision of up to 4 digits after the decimal point
- RustAnalyzer Lsp autocompletion + Function documentation template creations
