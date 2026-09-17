// Foreign sbt layout marker: adopted without upstream changes. The Scala
// Gazelle extension reads only `.scala` sources; this file records the
// foreign sbt coordinates without participating in Bazel resolution
// (Coursier-backed lock resolution stays open under #7).
ThisBuild / scalaVersion := "2.13.18"
ThisBuild / organization := "example.com"
lazy val root = (project in file(".")).aggregate(greet, solo)
lazy val greet = project
lazy val solo = project
