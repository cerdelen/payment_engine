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


AI Usage
- writing serde serializer for decimal points with a precision of up to 4 digits after the decimal point
- RustAnalyzer Lsp autocompletion + Function documentation template creations
