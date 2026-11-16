# Experimental numeric presentation patterns, not embedded in the product.
# Own complete sign/unit placement here; code supplies unsigned prepared digits.

# Short complete ordinary decimal quantity; preserve a negative zero sign.
number-cost-quantity =
    { $sign ->
        [negative] -{ $absolute }
       *[positive] { $absolute }
    }

# Short complete USD amount; $absolute already has exactly two fraction digits.
number-cost-usd =
    { $sign ->
        [negative] -${ $absolute }
       *[positive] ${ $absolute }
    }

# Short complete compact amount; $absolute is prepared digits, $sign preserves
# signed zero, and $exponent is the language-owned decimal scaling exponent.
number-cost-compact =
    { $exponent ->
        [3] { $sign ->
            [negative] -{ $absolute }K
           *[positive] { $absolute }K
        }
        [6] { $sign ->
            [negative] -{ $absolute }M
           *[positive] { $absolute }M
        }
        [9] { $sign ->
            [negative] -{ $absolute }B
           *[positive] { $absolute }B
        }
        [12] { $sign ->
            [negative] -{ $absolute }T
           *[positive] { $absolute }T
        }
       *[0] { $sign ->
            [negative] -{ $absolute }
           *[positive] { $absolute }
        }
    }
