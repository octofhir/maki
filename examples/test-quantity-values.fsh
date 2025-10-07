// Test Quantity Values - Task #009
Profile: QuantityTestProfile
Parent: Observation
Id: quantity-test-profile
Title: "Quantity Test Profile"
Description: "Test profile for Quantity value parsing"
* valueQuantity = 5.4 'mg' "milligrams"
* component[0].valueQuantity = 120 'mm[Hg]' "millimeters of mercury"
* component[1].valueQuantity = 80 'mm[Hg]'
* component[2].valueQuantity = 10.5 'g/dL'

// Test without unit string
* component[3].valueQuantity = 100 'kg'

// Test with complex UCUM unit
* component[4].valueQuantity = 22.3 '%' "%"
* component[5].valueQuantity = 88.5 '10*3/uL' "10*3/uL"
