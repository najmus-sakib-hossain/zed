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


As you can see from the screenshot that there is 2 problem in our chat input:
1. The 
```

# Chat Input Top
```markdown
In our chat session at the bottom right now, we are showing 5 icons instead of showing them in the bottom right. Please show them in the bottom center. And for now on, please hide the like and dislike icon from there. 
```

# File Explorer Sidebar
```markdown
Currently in the world File Explorer sidebar at our code editor there is horizontal scroll, but horizontal scroll on File Explorer looks very bad. So please make sure to truncate the files and folder names instead of giving horizontal scroll, and there shouldn't be any horizontal scroll in our File Explorer sidebar. 
```

# File Explorer Sidebar
```markdown
Currently in the world File Explorer sidebar at our code editor there is horizontal scroll, but horizontal scroll on File Explorer looks very bad. So please make sure to truncate the files and folder names instead of giving horizontal scroll, and there shouldn't be any horizontal scroll in our File Explorer sidebar. 
```

# Model picker free models addition
```markdown
Currently in the model picker currently there are provider as a labels put please make the provider labels as options and do these:
1. In the providers labels left provider text please make the text bigger so that the provider names is more visible
2. In the providers label right please add the numbers of the models supported by that provdiers as a badge number
3. Int the provider label most rigth please add collapse and expand icon so that when we click on that icon the models under that provider will be shown and when we click again it will be hidden. Please make sure to use suitable icons from lucide-pack using our icon command

And in our model picker please add support for free models via our crates/providers crate - there I current added 4 free models from opencode and there are still 3 free models that we can use so please learn from the details of the crates/providers/README.md file and add 2 providers like the opencode.rs called - mlvoca.rs and pollinations.rs add support for these models:
  "pollinations": {
        "api_url": "https://text.pollinations.ai/openai",
        "available_models": [
          {
            "name": "openai-fast",
            "display_name": "OpenAI Fast",
            "max_tokens": 131000,
            "max_output_tokens": 32768,
          },
        ],
      },
      "mlvoca": {
        "api_url": "https://mlvoca.com",
        "available_models": [
          {
            "name": "tinyllama",
            "display_name": "TinyLlama",
            "max_tokens": 131000,
            "max_output_tokens": 32768,
          },
          {
            "name": "deepseek-r1:1.5b",
            "display_name": "DeepSeek R1 1.5B",
            "max_tokens": 131000,
            "max_output_tokens": 32768,
          },
        ],
      },
      
And in our ai providers selector top there should be a new providers called "Free" and in the right of the provider label there should 7 badge and and in the most right of the provider label there should be collapse and expand icon so that when we click on that icon the models under that provider will be shown and when we click again it will be hidden. Please make sure to use suitable icons from lucide-pack using our icon command - Please make those 7 free models functional and integrate in our code editor correctly!!! And we will show those 7 free models everytime even there user didn't configured any providers but those 7 models will show correctly!!!
```
