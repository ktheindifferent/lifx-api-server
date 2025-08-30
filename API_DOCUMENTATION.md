# LIFX API Server - New Features Documentation

## Overview
This document describes the newly implemented LIFX API features including Effects, Scenes, Cycle, and Clean APIs.

## Effects API

### Pulse Effect
**Endpoint:** `POST /v1/lights/:selector/effects/pulse`

Pulses lights between two colors.
