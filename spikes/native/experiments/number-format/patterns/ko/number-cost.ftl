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
        [negative] -US${ $absolute }
       *[positive] US${ $absolute }
    }

# Short complete compact amount; $absolute is prepared digits, $sign preserves
# signed zero, and $exponent is the language-owned decimal scaling exponent.
number-cost-compact =
    { $exponent ->
        [3] { $sign ->
            [negative] -{ $absolute }천
           *[positive] { $absolute }천
        }
        [4] { $sign ->
            [negative] -{ $absolute }만
           *[positive] { $absolute }만
        }
        [8] { $sign ->
            [negative] -{ $absolute }억
           *[positive] { $absolute }억
        }
        [12] { $sign ->
            [negative] -{ $absolute }조
           *[positive] { $absolute }조
        }
       *[0] { $sign ->
            [negative] -{ $absolute }
           *[positive] { $absolute }
        }
    }
