This is the current status of our code editor chatbot. Now we have to work on it to change some stuff. From the chatbot now for starters, please explain what you can see so that we can do next steps.
You are mostly correctly but wrong about many thing - now try to understand what I am about to say and if don't understand than instead of doing any stupid stuffs:

# ChatInput
```markdown
1. So, current the placeholder text is in the top of the chat input please make it one line bottom and ther in the left please use our icon library lucide packs attact icon and download if not already present in aseets folder and then please put that icon in the top left of the chat input

2. And in the bottom there before "Ai profiles selector like Ask, Write and Minimal" if one chat session we are showing a context pie chart like circle please move that from there and put in the chat input top rigth zoom icon and make sure to put in the top rigth of chat input and it should show everything time not only in chat sessions

3. Now, in the chat input bottom there is Ai profile, Ai model selector please more those to the chat input bottom left and in there please also add a new selector called "Target selector" in there please list these:
1. Local
2. Background
3. Cloud
And pleas use selector icon and also please use suitable icons from lucide-pack using our icon commamnd

4. In the top right of the chat input please put another icon called "Enhance" and it will enhance the prompt so in the top right of the chat input there will be three items - zoom icon, enhance icon and context pie chart icon. Please make sure to put them in the right order and also make sure to use suitable icons from lucide-pack using our icon command

5. Now as we moved the context pie chart to the top right of the chat input and also added enhance icon there, And also moved the add in the top left of the chat input so there is - Follow Zed agent button, Agent Profile selector, Ai model selector, And submit/stop button

Now please make the bottom of the chat input like this - on the chat bottom left there should be: Ai Target that we newly added, Ai profile selector and Ai model selector and on the chat bottom right there should be The follow zed agent button and submit/stop button!!!

So, I have added two screenshots in the chat, and from the first screenshot you can look that there are two problems.
1. The message input in chat is starting from the top, but as we introduce top icons, we need to give gaps at the top of the chat input. And also the channel input has a right side gap. Please remove that right side gap, as previously it was having that gap for the Zoom icon, but as currently the Zoom icon will be on top, so we don't need to contain our chat text input. 
2. Currently the ```Icon Target:Local select option Icon``` is showing in the chat-input bottom left but please show ```Icon And Select Option Icon``` There and also currently the Ai profiles like ```Ask, Write and Minimal``` are as text so please download lucide pack icons for those and on selection just show the icons instead of text and also please make sure to use suitable icons from lucide-pack using our icon command and when showing option menu then they should both have icons and text but when we select them then only icons should be shown in the chat input bottom left and also please make sure to use suitable icons from lucide-pack using our icon command
3. In the chat input bottom left 3 selectors please use border by default and when we select any option from those selectors then please make the border color to primary color and also make the background color to primary color with 10% opacity and also change the text color to primary color and also make the icons color to primary color and when not selected then it should be default border and text and icon colors. Please make sure to use suitable icons from lucide-pack using our icon command
