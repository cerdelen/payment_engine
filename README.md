# Payments engine
Assumptions made
- As floating point numbers are not totally accurate we cannot use f64 or f32 for the amount fields. Instead I will be using a scaled i128 which should be enough as:
    - maximum positive value is (2^127 - 1) / 10 ^ 4
    - maximum negative value is - (2^127) / 10 ^ 4
This should be more than enough to handle any reasonable transfer amounts


Amt values are fractional with a precision of up to 4 after the decimal point. Allowed formats are "x", "x.x", "x.xx", "x.xxx", "x.xxxx".
The whole number string will be accepted up to the aformentioned over/underflow boundries

Output normalization will always include a decimal point



AI Usage
- writing serde serializer for decimal points with a precision of up to 4 digits after the decimal point
- RustAnalyzer Lsp autocompletion + Function documentation template creations
