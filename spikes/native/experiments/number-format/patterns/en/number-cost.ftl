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

# Short complete compact amount. $category is computed by the maintained
# plural engine from the rounded coefficient; every CLDR category is declared
# so another language can specialize a whole amount without a Rust edit.
number-cost-compact =
    { $category ->
        [zero] { number-cost-compact-body }
        [one] { number-cost-compact-body }
        [two] { number-cost-compact-body }
        [few] { number-cost-compact-body }
        [many] { number-cost-compact-body }
       *[other] { number-cost-compact-body }
    }

# Complete default compact amount, reused above. $absolute is prepared digits;
# $sign preserves signed zero; numeric keys declare this folder's scale data.
number-cost-compact-body =
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
