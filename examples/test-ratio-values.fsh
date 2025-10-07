// Test Ratio Values - Task #010
Profile: RatioTestProfile
Parent: Observation
Id: ratio-test-profile
Title: "Ratio Test Profile"
Description: "Test profile for Ratio value parsing"

// Simple ratio: number : number
* valueRatio = 1:128

// Ratio with quantities
* component[0].valueRatio = 5 'mg':10 'mL'

// Ratio with decimal numbers
* component[1].valueRatio = 1.5:2.5
