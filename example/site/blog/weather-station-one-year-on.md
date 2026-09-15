---
{
    "title": "The weather station, one year on",
    "author": "Wren Calloway",
    "description": "What survived a year on the garden fence, what corroded, and what the data showed",
    "date": "2026-01-24",
    "tags": ["hardware", "electronics"],
    "draft": false
}
---

A year ago I bolted a [homemade weather station](/projects/hardware/weather-station/) to
the back fence. It has logged a reading every five minutes since, give or take one very
long power cut in October.

![The station: a white radiation shield on a post, with a solar panel above it](/assets/images/projects/weather-station.svg)

## What held up

The solar panel and the battery pack are the stars. The 18650 cells never dropped below
40% even in December, which surprised me. The 3D-printed radiation shield has yellowed
but still keeps the temperature sensor honest.

## What did not

The anemometer bearings seized in August. Cheap sealed bearings are not sealed against
fine dust, it turns out. I replaced them with stainless ones and added a drip edge above
the housing.

The humidity sensor also drifted about 6% high over the summer. I now recalibrate it
against a salt test every spring.

## The data

The most interesting number: the fence is on average 1.8 °C warmer than the nearest
airport station, and up to 5 °C warmer on still summer evenings. That is the brick wall
next to it, radiating the day's heat back out.

Next year's project is a rain gauge that does not fill up with leaves.
