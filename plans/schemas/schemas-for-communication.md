I want to create a webrtc application where users can also send chat messages to each 
other.  The app has two main business use cases. The first is as a TTRPG tool so that the
Game Master (GM) and players can interact with each other.  It is possible that an AI
agent can be the GM.  The other use case is for a meeting with at least 2 users, one of
which should be the host.  In either case, there is not only the audio and video data
that is exchanged from the WebRTC, but also the chat interface.  Users should be allowed
to choose if a message is broadcast publicly, or can privately send messages only to 
other specific members in the session.

The app should save the messages to some kind of durable log, like apache iggy. The 
message and metadata should have schema in Avro format that will be used to persist the
messages.  The storage location should be configurable, such as locally, or to a remote
S3 location for each topic.  Other events from the application like connection events
should have their own schemas and topics.  Also keep in mind that the schemas may evolve
over time, so we want to keep track off the schema version a message or event was created
against.

Since I would like to use apache iggy, and it will be a webrtc based app, rust and wasm
should be used for the tech stack.  The leptos framework can be used for both the front 
and the back end. Use pixi as the project manager, because we will later tie in some
Machine Learning applications for the data that is generated.

Can you create an initial build out for this webrtc app, and in particular consider the
different kinds of messages and events we would like to keep track of from an analytics
and machine learning point of view.  