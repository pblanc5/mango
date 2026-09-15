---
{
    "title": "Backyard weather station",
    "author": "Wren Calloway",
    "description": "A solar-powered station that logs temperature, humidity, pressure and wind every five minutes",
    "date": "2025-06-01",
    "tags": ["hardware", "electronics"],
    "draft": false
}
---

![The station: a white radiation shield on a post, with a solar panel above it](/assets/images/projects/weather-station.svg)

**Status:** running since January 2025 · **Cost:** about $90

## Parts

| Part                         | Job                          |
|------------------------------|------------------------------|
| ESP32 board                  | Reads sensors, sends data over Wi-Fi |
| BME280                       | Temperature, humidity, pressure |
| Cup anemometer (reed switch) | Wind speed                   |
| 6 V solar panel + 2× 18650   | Power, day and night         |
| 3D-printed radiation shield  | Keeps the sun off the sensor |

## Software

The board wakes every five minutes, takes a reading, posts it to a tiny server on my home
network and goes back to deep sleep. Average current draw is under 2 mA, which is why the
battery survives winter.

The server stores readings in SQLite and draws a chart of the last week. Nothing fancy,
and nothing in the cloud.

## Lessons

One year of results, including the bearings that did not survive the summer, are in
[The weather station, one year on](/blog/weather-station-one-year-on/).
