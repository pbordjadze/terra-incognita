import QuartzCore

/// Something a native prototype draws over the globe in step with its camera (`PrototypeStage.follow`):
/// its layer lies between the globe's drawing and its names, below the markers; it is told each frame
/// the globe ticks, after the camera has moved for it; and it is taken away when the prototype ends.
/// The clouds (`Clouds/`) are the first.
protocol GlobeFollower: AnyObject {
    var layer: CALayer { get }
    /// A frame of the globe's: `globe.stageCamera` is where it is now.
    func frame(of globe: GlobeView)
    /// The prototype has ended: let go of everything (its layer is removed for it).
    func stop()
}
